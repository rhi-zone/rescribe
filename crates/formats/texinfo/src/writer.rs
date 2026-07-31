#![allow(clippy::collapsible_if)]
//! Streaming Texinfo writer — converts a stream of events directly to
//! Texinfo text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::emit`]. It is a second, independent
//! emission path from the tree-based `emit()` function, not a thin wrapper
//! around it.
//!
//! # Buffer model
//!
//! There is exactly **one** growing output buffer (`Writer::out`) for the
//! whole document. Frames on the `Vec<Frame>` stack (`O(nesting depth)`) hold
//! only a `usize` mark into `out` plus a couple of small scalars — never a
//! copy of accumulated content. Children write **straight through** into
//! `out`.
//!
//! Every Texinfo construct turns out to be **write-straight-through**: unlike
//! RST (heading underline width) or a Markdown-style table (column widths),
//! nothing in Texinfo's own emit logic computes a prefix from content that
//! hasn't been seen yet — heading commands are chosen from `level`/`kind`
//! (known at `StartHeading`), list/table markers are chosen from `ordered`/
//! `is_header` (known at the container's `Start*`), and `@multitable` needs
//! no column-width pass at all (unlike RST's grid table). The three
//! subtleties that exist are all resolved by *unconditional* writes plus a
//! flag, never buffering:
//!
//! - **`ListItem`/`TableCell`/`DefinitionTerm`/etc. in an invalid context**
//!   (e.g. a stray `EndListItem` with no enclosing `List`): the opening
//!   marker is written unconditionally when the frame opens, then discarded
//!   by truncating `out` back to the frame's mark if the *closing* event
//!   finds an invalid parent — the same "write first, truncate on invalid
//!   parent" pattern `rst-fmt`'s `Writer` uses (see its module doc).
//! - **`TableCell`'s `" @tab "` separator**: whether to emit it is known the
//!   instant `StartTableCell` fires (it's "not the row's first cell"),
//!   tracked as a `cell_count` scalar on the parent `TableRow` frame.
//! - **`Link`'s `", "` before its optional link text**: `@uref{url` is
//!   written immediately at `StartLink`; the `", "` is written lazily, right
//!   before the *first* content byte a child of the link contributes (tracked
//!   via a `wrote_any` bool on the `Link` frame) — so an empty-text link
//!   (`@uref{url}`) costs nothing extra and a non-empty one never buffers its
//!   text separately to learn whether it was empty.
//!
//! The document header (`\input texinfo` / `@setfilename` / `@settitle` /
//! `@node Top` / `@top`) needs the title before writing any block content.
//! `events()`'s own contract guarantees `Event::Title` (if present) is
//! emitted at most once, before any block event — so the header is written
//! lazily on the *first* event this `Writer` ever sees: if it's `Title`, the
//! header is written with that title and the event itself produces no
//! further output; otherwise the header is written with no title before that
//! first event is processed. No event is ever buffered to make this
//! decision.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity) as soon as the frame stack empties, mirroring `rst-fmt`.
//! Memory is `O(largest top-level block + nesting depth)`, not `O(full
//! document)`.
//!
//! # Example
//! ```no_run
//! use texinfo::writer::Writer;
//! use texinfo::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1, kind: texinfo::HeadingKind::Numbered });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::ast::{CodeBlockVariant, CrossRefKind, HeadingKind, SymbolKind};
use crate::events::Event;
use std::io::Write;

/// Streaming Texinfo writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to write the document footer, flush the
/// remainder, and recover the sink.
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Cleared (capacity retained) after
    /// each top-level block is flushed.
    out: String,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level.
    stack: Vec<Frame>,
    /// Whether the document header has been written yet (decided on the
    /// first event seen — see the module doc).
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
            header_written: false,
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level block.
    pub fn write_event(&mut self, event: Event<'_>) {
        if !self.header_written {
            self.header_written = true;
            if let Event::Title(title) = event {
                self.write_header(Some(&title));
                self.maybe_flush();
                return;
            }
            self.write_header(None);
        }
        self.process(event);
        self.maybe_flush();
    }

    /// Write the document footer, flush any remaining buffered output, and
    /// recover the underlying sink.
    pub fn finish(mut self) -> W {
        if !self.header_written {
            self.header_written = true;
            self.write_header(None);
        }
        self.out.push_str("\n@bye\n");
        self.flush();
        self.sink
    }

    fn write_header(&mut self, title: Option<&str>) {
        self.out.push_str("\\input texinfo\n");
        self.out.push_str("@setfilename output.info\n");
        if let Some(title) = title {
            self.out.push_str("@settitle ");
            self.out.push_str(title);
            self.out.push('\n');
        }
        self.out.push_str("\n@node Top\n");
        if let Some(title) = title {
            self.out.push_str("@top ");
            self.out.push_str(title);
            self.out.push_str("\n\n");
        }
    }

    // ── Buffer primitives ─────────────────────────────────────────────────

    fn flush(&mut self) {
        if !self.out.is_empty() {
            let _ = self.sink.write_all(self.out.as_bytes());
            self.out.clear();
        }
    }

    /// Flush if this event completed the last open construct (top-level).
    fn maybe_flush(&mut self) {
        if self.stack.is_empty() {
            self.flush();
        }
    }

    /// Whether the top-of-stack frame accepts block children (mirrors the
    /// old `DocBuilder::push_block`'s match: only `Document`/`Blockquote`/
    /// `DefinitionDesc`/`Float` ever collected blocks).
    fn accepts_blocks(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(
                Frame::Blockquote { .. } | Frame::DefinitionDesc { .. } | Frame::FloatMark { .. }
            )
        )
    }

    /// Whether the top-of-stack frame accepts inline children (mirrors the
    /// old `DocBuilder::push_inline`'s match).
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::Heading { .. }
                    | Frame::ListItem { .. }
                    | Frame::DefinitionTerm { .. }
                    | Frame::TableCell
                    | Frame::Inline { .. }
                    | Frame::Link { .. }
            )
        )
    }

    /// Whether an "either Paragraph child gets `\n` instead of `\n\n`"
    /// context is on top — `Blockquote`/`DefinitionDesc` in the old
    /// `emit_block`'s special-cased loop over their children.
    fn tight_paragraph_context(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(Frame::Blockquote { .. } | Frame::DefinitionDesc { .. })
        )
    }

    /// Called right before the first content byte a `Link` child would
    /// contribute — writes the `", "` separator lazily, exactly once.
    fn before_inline_content(&mut self) {
        if let Some(Frame::Link { wrote_any, .. }) = self.stack.last_mut() {
            if !*wrote_any {
                *wrote_any = true;
                self.out.push_str(", ");
            }
        }
    }

    /// Write a leaf inline construct's full text, gated on
    /// [`accepts_inline`](Self::accepts_inline) — mirrors every
    /// `push_inline(Inline::Foo(...))` call in the old AST-reconstruction
    /// path.
    fn write_inline(&mut self, parts: &[&str]) {
        if !self.accepts_inline() {
            return;
        }
        self.before_inline_content();
        for p in parts {
            self.out.push_str(p);
        }
    }

    fn write_inline_escaped(&mut self, prefix: &str, s: &str, suffix: &str) {
        if !self.accepts_inline() {
            return;
        }
        self.before_inline_content();
        self.out.push_str(prefix);
        write_escaped(&mut self.out, s);
        self.out.push_str(suffix);
    }

    /// Write a leaf block-level construct, gated on
    /// [`accepts_blocks`](Self::accepts_blocks).
    fn write_block_leaf(&mut self, parts: &[&str]) {
        if !self.accepts_blocks() {
            return;
        }
        for p in parts {
            self.out.push_str(p);
        }
    }

    /// Open an inline span whose opening text is already fully known: write
    /// it straight through and record the mark needed to undo it if the
    /// span turns out to have no valid enclosing context.
    fn open_inline_span(&mut self, open: &str) {
        let mark = self.out.len();
        if self.accepts_inline() {
            self.before_inline_content();
        }
        self.out.push_str(open);
        self.stack.push(Frame::Inline { mark });
    }

    fn close_inline_span(&mut self) {
        if let Some(Frame::Inline { mark }) = self.stack.pop() {
            self.out.push('}');
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
                    if self.tight_paragraph_context() {
                        self.out.push('\n');
                    } else {
                        self.out.push_str("\n\n");
                    }
                    if !self.accepts_blocks() {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartHeading { level, kind } => {
                let mark = self.out.len();
                let command = heading_command(level, kind);
                if self.accepts_blocks() {
                    self.out.push_str(command);
                    self.out.push(' ');
                }
                self.stack.push(Frame::Heading { mark });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading { mark }) = self.stack.pop() {
                    if self.accepts_blocks() {
                        self.out.push_str("\n\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartBlockquote => {
                let mark = self.out.len();
                if self.accepts_blocks() {
                    self.out.push_str("@quotation\n");
                }
                self.stack.push(Frame::Blockquote { mark });
            }
            Event::EndBlockquote => {
                if let Some(Frame::Blockquote { mark }) = self.stack.pop() {
                    if self.accepts_blocks() {
                        self.out.push_str("@end quotation\n\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartList { ordered } => {
                let mark = self.out.len();
                if self.accepts_blocks() {
                    self.out.push_str(if ordered {
                        "@enumerate\n"
                    } else {
                        "@itemize @bullet\n"
                    });
                }
                self.stack.push(Frame::List { ordered, mark });
            }
            Event::EndList => {
                if let Some(Frame::List { ordered, mark }) = self.stack.pop() {
                    if self.accepts_blocks() {
                        self.out.push_str(if ordered {
                            "@end enumerate\n\n"
                        } else {
                            "@end itemize\n\n"
                        });
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartListItem => {
                let mark = self.out.len();
                let in_list = matches!(self.stack.last(), Some(Frame::List { .. }));
                if in_list {
                    self.out.push_str("@item ");
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
            Event::CodeBlock { variant, content } => {
                if self.accepts_blocks() {
                    let (start, end) = code_block_delims(variant);
                    self.out.push_str(start);
                    self.out.push('\n');
                    self.out.push_str(&content);
                    if !content.ends_with('\n') {
                        self.out.push('\n');
                    }
                    self.out.push_str(end);
                    self.out.push_str("\n\n");
                }
            }
            Event::StartDefinitionList => {
                let mark = self.out.len();
                if self.accepts_blocks() {
                    self.out.push_str("@table @asis\n");
                }
                self.stack.push(Frame::DefinitionList { mark });
            }
            Event::EndDefinitionList => {
                if let Some(Frame::DefinitionList { mark }) = self.stack.pop() {
                    if self.accepts_blocks() {
                        self.out.push_str("@end table\n\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartDefinitionTerm => {
                let mark = self.out.len();
                let in_list = matches!(self.stack.last(), Some(Frame::DefinitionList { .. }));
                if in_list {
                    self.out.push_str("@item ");
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
            Event::StartTable => {
                let mark = self.out.len();
                if self.accepts_blocks() {
                    self.out.push_str("@multitable\n");
                }
                self.stack.push(Frame::Table { mark });
            }
            Event::EndTable => {
                if let Some(Frame::Table { mark }) = self.stack.pop() {
                    if self.accepts_blocks() {
                        self.out.push_str("@end multitable\n\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartTableRow { is_header } => {
                let mark = self.out.len();
                let in_table = matches!(self.stack.last(), Some(Frame::Table { .. }));
                if in_table {
                    self.out
                        .push_str(if is_header { "@headitem " } else { "@item " });
                }
                self.stack.push(Frame::TableRow {
                    mark,
                    cell_count: 0,
                });
            }
            Event::EndTableRow => {
                if let Some(Frame::TableRow { mark, .. }) = self.stack.pop() {
                    if matches!(self.stack.last(), Some(Frame::Table { .. })) {
                        self.out.push('\n');
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartTableCell => {
                if let Some(Frame::TableRow { cell_count, .. }) = self.stack.last_mut() {
                    if *cell_count > 0 {
                        self.out.push_str(" @tab ");
                    }
                    *cell_count += 1;
                }
                self.stack.push(Frame::TableCell);
            }
            Event::EndTableCell => {
                self.stack.pop();
            }
            Event::StartMenu => {
                let mark = self.out.len();
                if self.accepts_blocks() {
                    self.out.push_str("@menu\n");
                }
                self.stack.push(Frame::Menu { mark });
            }
            Event::EndMenu => {
                if let Some(Frame::Menu { mark }) = self.stack.pop() {
                    if self.accepts_blocks() {
                        self.out.push_str("@end menu\n\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::MenuEntry { node, description } => {
                if matches!(self.stack.last(), Some(Frame::Menu { .. })) {
                    self.out.push_str("* ");
                    self.out.push_str(&node);
                    self.out.push_str("::");
                    if let Some(desc) = &description {
                        self.out.push(' ');
                        self.out.push_str(desc);
                    }
                    self.out.push('\n');
                }
            }
            Event::HorizontalRule => {
                self.write_block_leaf(&["\n@sp 1\n@noindent\n@center * * *\n@sp 1\n\n"]);
            }
            Event::RawBlock {
                environment,
                content,
            } => {
                if self.accepts_blocks() {
                    self.out.push('@');
                    self.out.push_str(&environment);
                    self.out.push('\n');
                    self.out.push_str(&content);
                    if !content.ends_with('\n') {
                        self.out.push('\n');
                    }
                    self.out.push_str("@end ");
                    self.out.push_str(&environment);
                    self.out.push_str("\n\n");
                }
            }
            Event::StartFloat { float_type, label } => {
                let mark = self.out.len();
                if self.accepts_blocks() {
                    self.out.push_str("@float");
                    if let Some(ft) = &float_type {
                        self.out.push(' ');
                        self.out.push_str(ft);
                        if let Some(lb) = &label {
                            self.out.push(',');
                            self.out.push_str(lb);
                        }
                    }
                    self.out.push('\n');
                }
                self.stack.push(Frame::FloatMark { mark });
            }
            Event::EndFloat => {
                if let Some(Frame::FloatMark { mark }) = self.stack.pop() {
                    if self.accepts_blocks() {
                        self.out.push_str("@end float\n\n");
                    } else {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::NoIndent => {
                self.write_block_leaf(&["@noindent\n"]);
            }

            // ── Inline events ───────────────────────────────────────────
            Event::Text(cow) => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    write_escaped(&mut self.out, &cow);
                }
            }
            Event::SoftBreak => self.write_inline(&[" "]),
            Event::LineBreak => self.write_inline(&["@*\n"]),
            Event::StartStrong => self.open_inline_span("@strong{"),
            Event::EndStrong => self.close_inline_span(),
            Event::StartEmphasis => self.open_inline_span("@emph{"),
            Event::EndEmphasis => self.close_inline_span(),
            Event::InlineCode(cow) => self.write_inline_escaped("@code{", &cow, "}"),
            Event::StartVar => self.open_inline_span("@var{"),
            Event::EndVar => self.close_inline_span(),
            Event::File(cow) => self.write_inline_escaped("@file{", &cow, "}"),
            Event::Command(cow) => self.write_inline_escaped("@command{", &cow, "}"),
            Event::Option(cow) => self.write_inline_escaped("@option{", &cow, "}"),
            Event::Env(cow) => self.write_inline_escaped("@env{", &cow, "}"),
            Event::Samp(cow) => self.write_inline_escaped("@samp{", &cow, "}"),
            Event::Kbd(cow) => self.write_inline_escaped("@kbd{", &cow, "}"),
            Event::Key(cow) => self.write_inline_escaped("@key{", &cow, "}"),
            Event::StartDfn => self.open_inline_span("@dfn{"),
            Event::EndDfn => self.close_inline_span(),
            Event::Cite(cow) => self.write_inline_escaped("@cite{", &cow, "}"),
            Event::Acronym { abbrev, expansion } => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push_str("@acronym{");
                    self.out.push_str(&abbrev);
                    if let Some(exp) = &expansion {
                        self.out.push_str(", ");
                        self.out.push_str(exp);
                    }
                    self.out.push('}');
                }
            }
            Event::Abbr { abbrev, expansion } => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push_str("@abbr{");
                    self.out.push_str(&abbrev);
                    if let Some(exp) = &expansion {
                        self.out.push_str(", ");
                        self.out.push_str(exp);
                    }
                    self.out.push('}');
                }
            }
            Event::Roman(cow) => self.write_inline_escaped("@r{", &cow, "}"),
            Event::SmallCaps(cow) => self.write_inline_escaped("@sc{", &cow, "}"),
            Event::StartDirectItalic => self.open_inline_span("@i{"),
            Event::EndDirectItalic => self.close_inline_span(),
            Event::StartDirectBold => self.open_inline_span("@b{"),
            Event::EndDirectBold => self.close_inline_span(),
            Event::DirectTypewriter(cow) => self.write_inline_escaped("@t{", &cow, "}"),
            Event::StartLink { url } => {
                let mark = self.out.len();
                if self.accepts_inline() {
                    self.before_inline_content();
                }
                self.out.push_str("@uref{");
                self.out.push_str(&url);
                self.stack.push(Frame::Link {
                    mark,
                    wrote_any: false,
                });
            }
            Event::EndLink => {
                if let Some(Frame::Link { mark, .. }) = self.stack.pop() {
                    self.out.push('}');
                    if !self.accepts_inline() {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::Image {
                file,
                width,
                height,
                alt,
                extension,
            } => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push_str("@image{");
                    self.out.push_str(&file);
                    if width.is_some() || height.is_some() || alt.is_some() || extension.is_some() {
                        self.out.push(',');
                        if let Some(w) = &width {
                            self.out.push_str(w);
                        }
                        self.out.push(',');
                        if let Some(h) = &height {
                            self.out.push_str(h);
                        }
                        self.out.push(',');
                        if let Some(a) = &alt {
                            self.out.push_str(a);
                        }
                        self.out.push(',');
                        if let Some(e) = &extension {
                            self.out.push_str(e);
                        }
                    }
                    self.out.push('}');
                }
            }
            Event::StartSuperscript => self.open_inline_span("@sup{"),
            Event::EndSuperscript => self.close_inline_span(),
            Event::StartSubscript => self.open_inline_span("@sub{"),
            Event::EndSubscript => self.close_inline_span(),
            Event::StartFootnoteDef => self.open_inline_span("@footnote{"),
            Event::EndFootnoteDef => self.close_inline_span(),
            Event::CrossRef { kind, node, text } => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    let cmd = match kind {
                        CrossRefKind::Xref => "@xref",
                        CrossRefKind::Ref => "@ref",
                        CrossRefKind::Pxref => "@pxref",
                    };
                    self.out.push_str(cmd);
                    self.out.push('{');
                    self.out.push_str(&node);
                    if let Some(t) = &text {
                        self.out.push_str(", ");
                        self.out.push_str(t);
                    }
                    self.out.push('}');
                }
            }
            Event::Anchor { name } => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push_str("@anchor{");
                    self.out.push_str(&name);
                    self.out.push('}');
                }
            }
            Event::NoBreak(cow) => self.write_inline_escaped("@w{", &cow, "}"),
            Event::Email { address, text } => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push_str("@email{");
                    self.out.push_str(&address);
                    if let Some(t) = &text {
                        self.out.push_str(", ");
                        self.out.push_str(t);
                    }
                    self.out.push('}');
                }
            }
            Event::Symbol(kind) => {
                let cmd = match kind {
                    SymbolKind::Dots => "@dots{}",
                    SymbolKind::EndDots => "@enddots{}",
                    SymbolKind::Minus => "@minus{}",
                    SymbolKind::Copyright => "@copyright{}",
                    SymbolKind::Registered => "@registeredsymbol{}",
                    SymbolKind::LaTeX => "@LaTeX{}",
                    SymbolKind::TeX => "@TeX{}",
                    SymbolKind::Tie => "@tie{}",
                };
                self.write_inline(&[cmd]);
            }

            // ── Document metadata ───────────────────────────────────────
            // Only ever meaningful as the very first event, handled in
            // write_event() before reaching process(); a Title arriving
            // later (violating events()'s own documented "at most once,
            // before any block events" contract) has no well-defined header
            // slot left to fill and is dropped.
            Event::Title(_) => {}
        }
    }
}

fn heading_command(level: u8, kind: HeadingKind) -> &'static str {
    match (level, kind) {
        (1, HeadingKind::Numbered) => "@chapter",
        (1, HeadingKind::Unnumbered) => "@unnumbered",
        (1, HeadingKind::Appendix) => "@appendix",
        (2, HeadingKind::Numbered) => "@section",
        (2, HeadingKind::Unnumbered) => "@unnumberedsec",
        (2, HeadingKind::Appendix) => "@appendixsec",
        (3, HeadingKind::Numbered) => "@subsection",
        (3, HeadingKind::Unnumbered) => "@unnumberedsubsec",
        (3, HeadingKind::Appendix) => "@appendixsubsec",
        (4, HeadingKind::Numbered) => "@subsubsection",
        (4, HeadingKind::Unnumbered) => "@unnumberedsubsubsec",
        (4, HeadingKind::Appendix) => "@appendixsubsubsec",
        _ => "@subsubsection",
    }
}

fn code_block_delims(variant: CodeBlockVariant) -> (&'static str, &'static str) {
    match variant {
        CodeBlockVariant::Example => ("@example", "@end example"),
        CodeBlockVariant::SmallExample => ("@smallexample", "@end smallexample"),
        CodeBlockVariant::Verbatim => ("@verbatim", "@end verbatim"),
        CodeBlockVariant::Lisp => ("@lisp", "@end lisp"),
        CodeBlockVariant::Display => ("@display", "@end display"),
        CodeBlockVariant::Format => ("@format", "@end format"),
    }
}

fn write_escaped(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '@' => out.push_str("@@"),
            '{' => out.push_str("@{"),
            '}' => out.push_str("@}"),
            _ => out.push(c),
        }
    }
}

/// Frames carry only a mark into the shared buffer and tiny scalars — never
/// accumulated content. `mark` is where this construct's output begins in
/// `Writer::out`, for discarding wholesale if it turns out to have no valid
/// enclosing context.
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
        ordered: bool,
        mark: usize,
    },
    ListItem {
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
    Table {
        mark: usize,
    },
    TableRow {
        mark: usize,
        cell_count: usize,
    },
    TableCell,
    Menu {
        mark: usize,
    },
    /// Named distinctly from `Event::StartFloat`/`EndFloat` only to avoid
    /// clashing with the module name `Float`; carries just the mark.
    FloatMark {
        mark: usize,
    },
    /// Any inline span whose closing text is the fixed `"}"` (every
    /// Texinfo braced-argument command) — `Strong`/`Emphasis`/`Var`/`Dfn`/
    /// `DirectItalic`/`DirectBold`/`Superscript`/`Subscript`/`FootnoteDef`
    /// all share this shape, so one variant covers all of them.
    Inline {
        mark: usize,
    },
    /// `@uref{url[, text]}` — the one inline span whose closing text depends
    /// on whether any child content arrived (see `before_inline_content`).
    Link {
        mark: usize,
        wrote_any: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedEvent;
    use std::borrow::Cow;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading {
            level: 1,
            kind: HeadingKind::Numbered,
        });
        w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("@chapter Hello"), "got: {s:?}");
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
        let input = "@chapter Hello\n\nA paragraph with @strong{bold} text.\n\n@itemize\n@item one\n@item two\n@end itemize\n";
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
    /// tree-based `emit()` for the same document — the guard that keeps the
    /// two independent emission paths honest.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "@chapter Hello\n\nSome intro with @strong{bold} and @code{x=1}.\n",
            "@section Sub\n\ntext with @emph{em} and @uref{http://x/, a link} here.\n",
            "@itemize\n@item one\n@item two\n@end itemize\n",
            "@enumerate\n@item first\n@item second\n@end enumerate\n",
            "@example\nsome code\n@end example\n",
            "@quotation\nA quoted paragraph.\n\nSecond para.\n@end quotation\n",
            "@table @asis\n@item term\ndefinition body\n@item term2\nanother\n@end table\n",
            "@multitable\n@headitem A @tab B\n@item Cell 1 @tab Cell 2\n@end multitable\n",
            "See @xref{Top}.\n",
            "@float\nFloat body.\n@end float\n",
            "@image{img.png,,,alt text,}\n",
            "A para with @email{a@@b.com, contact} link.\n",
            "@menu\n* Node One:: desc one\n* Node Two::\n@end menu\n",
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

    // Single shared tracking allocator for this module: `#[global_allocator]`
    // may be declared at most once for the whole test binary, so both the
    // allocation-count regression guard and the peak-memory/throughput
    // report below share it via module-level atomics rather than each
    // declaring their own.
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
        fn events_for(n: usize) -> Vec<OwnedEvent> {
            let mut evs = Vec::new();
            for i in 0..n {
                evs.push(OwnedEvent::StartHeading {
                    level: 2,
                    kind: HeadingKind::Numbered,
                });
                evs.push(OwnedEvent::Text(Cow::Owned(format!("Section {i}"))));
                evs.push(OwnedEvent::EndHeading);
                evs.push(OwnedEvent::StartParagraph);
                evs.push(OwnedEvent::Text(Cow::Owned("plain ".to_string())));
                evs.push(OwnedEvent::StartStrong);
                evs.push(OwnedEvent::Text(Cow::Owned("bold".to_string())));
                evs.push(OwnedEvent::EndStrong);
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
    /// document (many top-level sections). Run with
    /// `cargo test -p texinfo --release test_writer_peak_memory_and_throughput_report \
    ///  -- --ignored --nocapture` to see the numbers; `#[ignore]`d because it
    /// prints rather than asserts a specific threshold (peak-memory ratios
    /// are architecture-dependent, not a fixed regression gate — that's
    /// `test_writer_no_subtree_reconstruction_blowup`'s job).
    #[test]
    #[ignore]
    fn test_writer_peak_memory_and_throughput_report() {
        // A large synthetic Texinfo document: many top-level sections, each
        // with a heading, a paragraph with inline markup, and a short list —
        // built as real Texinfo source so both the streaming Writer (via
        // events()) and the builder (via parse()+emit()) process the exact
        // same content.
        fn synthetic_source(n: usize) -> String {
            let mut s = String::from("@settitle Big Document\n");
            for i in 0..n {
                s.push_str(&format!(
                    "@section Section {i}\n\n\
                     Some plain text with @strong{{bold}} and @emph{{italic}} markup, \
                     and a @uref{{http://example.com/{i}, link {i}}}.\n\n\
                     @itemize\n@item first point {i}\n@item second point {i}\n@end itemize\n\n"
                ));
            }
            s
        }

        /// A `Write` sink that counts bytes but never retains them — the
        /// point of measuring the streaming Writer's peak memory is to see
        /// *its own* internal state (bounded by the largest top-level
        /// block), not whatever a `Vec<u8>` sink would additionally
        /// accumulate. A real streaming caller (a file handle, a socket)
        /// doesn't retain the whole document either; a `Vec<u8>` sink would
        /// silently reintroduce O(full document) memory into the
        /// measurement regardless of how bounded the Writer itself is —
        /// exactly the harness artifact this comment warns against.
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

        let input = synthetic_source(5000);

        // IMPORTANT: `crate::events::events()` is itself not incremental for
        // this crate — `EventIter::new` (events.rs) eagerly runs `parse()`
        // to build the full AST, then walks it into a fully-materialized
        // `Vec<OwnedEvent>`, *before* the first event is ever yielded (a
        // separate, pre-existing gap in `events()`, out of scope for this
        // writer-focused change — see TODO.md). Calling `events(&input)`
        // inside the timed/tracked region would attribute that
        // whole-document materialization cost to the Writer, which is
        // exactly the kind of harness artifact that inflated rst-fmt's
        // figures earlier this session. So the event vec is built here,
        // *before* the tracked window opens, mirroring
        // `test_writer_no_subtree_reconstruction_blowup`'s `events_for()`
        // being called outside its own measured `run()`.
        let events: Vec<OwnedEvent> = crate::events::events(&input).collect();
        let (doc, _diags) = crate::parse::parse(&input);

        // Streaming Writer path: feed the pre-built event vec through
        // Writer alone, sink discarding.
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

        // Builder path: emit() alone, from the pre-built AST — the cost
        // shape the old buffer-events-then-reconstruct-the-AST Writer paid
        // on every document (see this module's doc comment for what
        // changed), now isolated the same way (AST built outside the
        // tracked window).
        let baseline = CURRENT.with(|c| c.get());
        PEAK.with(|p| p.set(baseline));
        let start = std::time::Instant::now();
        let built = crate::emit::emit(std::hint::black_box(&doc));
        let builder_elapsed = start.elapsed();
        let builder_peak = PEAK.with(|p| p.get()).saturating_sub(baseline);
        std::hint::black_box(&built);

        eprintln!(
            "texinfo streaming Writer vs parse()+emit() builder, {} bytes input, 5000 sections:\n\
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
}
