#![allow(clippy::collapsible_if)]
//! Streaming RST writer — converts a stream of events directly to RST text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::Block`]/[`crate::Inline`] value and
//! never calls [`crate::build_block`]/`build_inlines`/`build_inline`. Instead
//! each frame on its `Vec<Frame>` stack (`O(nesting depth)`) accumulates
//! **already-formatted text** (`String` buffers) as child events close; when
//! a construct's `End*` event arrives, the frame renders its own final text
//! once from those buffers (mirroring the `build_*` functions' output
//! byte-for-byte) and splices it into the parent frame's buffer — or writes
//! it straight to the sink if the stack is now empty. This is a second,
//! independent emission path from the tree-based `build()`/`emit()`
//! functions, not a thin wrapper around them: no intermediate `Block`/
//! `Inline` subtree is ever materialized. Memory is
//! `O(largest top-level block + nesting depth)`, not `O(full document)`.
//!
//! # Example
//! ```no_run
//! use rst_fmt::writer::Writer;
//! use rst_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::OwnedEvent;
use crate::{BuildContext, calculate_column_widths, emit_table_border};
use std::io::Write;

/// Accumulator for inline content that may itself be nested inside further
/// markup: a formatted-RST buffer (`buf`, e.g. `**bold**`) and a plain-text
/// buffer (`plain`) matching `collect_text_from_inlines`'s rules exactly
/// (delimiters never count as plain text). Only frames whose own contents
/// might need to contribute *plain* text further up the tree (inline
/// wrapping spans, which may end up nested inside anything, and `Heading`,
/// which needs its own plain-text length for the underline) carry this;
/// frames whose formatted output is a dead end (`Paragraph`, definition
/// list terms/descriptions, footnote defs) use a bare `String` instead —
/// tracking `plain` for those would be pure overhead, since nothing ever
/// reads it.
#[derive(Default)]
struct InlineAccum {
    buf: String,
    plain: String,
}

impl InlineAccum {
    fn new() -> Self {
        Self::default()
    }
}

/// One item's children in a list, tagged by dispatch kind so `EndListItem`
/// can replicate `build_list_item`'s per-child-kind formatting exactly.
enum ListChild {
    /// Formatted (unwrapped) inline buffer of a paragraph child.
    Paragraph(String),
    /// Already-fully-rendered text of a nested list child.
    NestedList(String),
    /// Already-fully-rendered text of any other block child.
    Other(String),
}

/// Where an inline contribution goes.
///
/// - `Full`: a frame that might itself be nested further up (inline spans,
///   `Heading`) — needs both the formatted buffer and the plain-text buffer.
/// - `BufOnly`: a frame whose formatted output is a dead end (`Paragraph`,
///   definition term/desc, footnote def) — plain text is never read back out
///   of these, so it is not tracked.
/// - `BufAndFlag`: like `BufOnly`, plus a presence flag (`Figure`'s caption,
///   which distinguishes "no caption content at all" from "caption content
///   that renders to an empty string" via `any`, not a plain-text length).
/// - `PlainOnly`: table cells — `build_table` never writes formatted markup
///   into a cell, only plain text.
/// - `None`: no enclosing inline-accepting frame; contribution is discarded.
enum Dest<'a> {
    Full(&'a mut InlineAccum),
    BufOnly(&'a mut String),
    BufAndFlag(&'a mut String, &'a mut bool),
    PlainOnly(&'a mut String),
    None,
}

/// Tag describing what kind of block just closed, so `push_block` can
/// replicate `build_list_item`'s per-child dispatch when the parent frame is
/// a `ListItem` (a `Paragraph` and a `List` are each formatted specially;
/// everything else is verbatim, matching the `other => build_block(other,
/// ctx)` arm).
enum BlockTag {
    Paragraph,
    List,
    Other,
}

/// Streaming RST writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to recover the sink once all events have been
/// fed.
pub struct Writer<W: Write> {
    sink: W,
    /// Frame stack for the block/inline subtree currently being assembled.
    /// Empty at top level — a `push_block` reaching an empty stack means the
    /// block just closed at the document's top level, so it is written
    /// immediately instead of being retained.
    stack: Vec<Frame>,
    /// Mirrors `BuildContext::list_depth`: incremented when a `List` frame is
    /// pushed (`StartList`), decremented when it is popped (`EndList`, right
    /// before that list's own output is finalized) — the same bracketing
    /// `build_list` uses around its item loop.
    list_depth: usize,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
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

    /// Dispatch a just-rendered block's text to whatever is enclosing it:
    /// another block container's buffer, the current list item's tagged
    /// child sequence, or straight to the sink if nothing encloses it.
    fn push_block(&mut self, text: String, tag: BlockTag) {
        match self.stack.last_mut() {
            Some(Frame::ListItem { children }) => {
                // A paragraph child keeps its raw (no trailing blank line)
                // form here — `render_list_item` appends the single "\n"
                // `build_list_item` uses for paragraph children, not the
                // "\n\n" a standalone paragraph gets elsewhere.
                children.push(match tag {
                    BlockTag::Paragraph => ListChild::Paragraph(text),
                    BlockTag::List => ListChild::NestedList(text),
                    BlockTag::Other => ListChild::Other(text),
                });
            }
            Some(
                Frame::Blockquote { buf } | Frame::Div { buf } | Frame::Admonition { buf, .. },
            ) => {
                buf.push_str(&text);
                if matches!(tag, BlockTag::Paragraph) {
                    buf.push_str("\n\n");
                }
            }
            None => {
                let _ = self.sink.write_all(text.as_bytes());
                if matches!(tag, BlockTag::Paragraph) {
                    let _ = self.sink.write_all(b"\n\n");
                }
            }
            _ => {} // unexpected context, discard
        }
    }

    /// Dispatch a leaf/closed inline's formatted+plain contribution to
    /// whatever inline-accumulating frame is on top of the stack.
    fn push_inline(&mut self, formatted: &str, plain: &str) {
        self.push_multi(&[formatted], plain);
    }

    /// Append each of `bufs` in order to the current top-of-stack frame's
    /// formatted buffer (direct `push_str` calls, no intermediate
    /// `format!`-allocated string), plus `plain` to its plain-text tracking
    /// (where the destination tracks plain text at all — see [`Dest`]).
    fn push_multi(&mut self, bufs: &[&str], plain: &str) {
        match self.dest() {
            Dest::Full(acc) => {
                for b in bufs {
                    acc.buf.push_str(b);
                }
                acc.plain.push_str(plain);
            }
            Dest::BufOnly(buf) => {
                for b in bufs {
                    buf.push_str(b);
                }
            }
            Dest::BufAndFlag(buf, any) => {
                for b in bufs {
                    buf.push_str(b);
                }
                *any = true;
            }
            Dest::PlainOnly(cell_plain) => cell_plain.push_str(plain),
            Dest::None => {} // unexpected context, discard
        }
    }

    /// Append `prefix + body + suffix` — shorthand for the common
    /// three-piece wrap (e.g. `"*"` + emphasis body + `"*"`).
    fn push_wrapped(&mut self, prefix: &str, body: &str, suffix: &str, plain: &str) {
        self.push_multi(&[prefix, body, suffix], plain);
    }

    /// The current top-of-stack frame's inline-accumulation target, if any.
    /// See [`Dest`] for what each variant means and why.
    fn dest(&mut self) -> Dest<'_> {
        match self.stack.last_mut() {
            Some(
                Frame::Emphasis(acc)
                | Frame::Strong(acc)
                | Frame::Strikeout(acc)
                | Frame::Underline(acc)
                | Frame::Subscript(acc)
                | Frame::Superscript(acc)
                | Frame::SmallCaps(acc)
                | Frame::Heading { acc, .. }
                | Frame::Link { acc, .. }
                | Frame::FootnoteDefInline { acc, .. }
                | Frame::Quoted { acc, .. }
                | Frame::RstSpan { acc, .. },
            ) => Dest::Full(acc),
            Some(
                Frame::Paragraph(buf)
                | Frame::DefinitionTerm(buf)
                | Frame::DefinitionDesc(buf)
                | Frame::FootnoteDef { buf, .. },
            ) => Dest::BufOnly(buf),
            Some(Frame::Figure {
                caption_buf,
                caption_any,
                ..
            }) => Dest::BufAndFlag(caption_buf, caption_any),
            Some(Frame::TableCell { plain }) => Dest::PlainOnly(plain),
            None
            | Some(
                Frame::Blockquote { .. }
                | Frame::List { .. }
                | Frame::ListItem { .. }
                | Frame::CodeBlock { .. }
                | Frame::Div { .. }
                | Frame::Table { .. }
                | Frame::TableRow { .. }
                | Frame::DefinitionList { .. }
                | Frame::Admonition { .. },
            ) => Dest::None,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OwnedEvent) {
        match event {
            // ── Block open/close ───────────────────────────────────────────────
            OwnedEvent::StartParagraph => {
                self.stack.push(Frame::Paragraph(String::new()));
            }
            OwnedEvent::EndParagraph => {
                if let Some(Frame::Paragraph(buf)) = self.stack.pop() {
                    // Note: no trailing "\n\n" appended here — `build_list_item`
                    // overrides paragraph formatting for list-item children
                    // (single "\n", no blank line), so the block-level "\n\n"
                    // is added in `push_block` only when the destination is
                    // NOT a list item.
                    self.push_block(buf, BlockTag::Paragraph);
                }
            }
            OwnedEvent::StartHeading { level } => {
                self.stack.push(Frame::Heading {
                    level,
                    acc: InlineAccum::new(),
                });
            }
            OwnedEvent::EndHeading => {
                if let Some(Frame::Heading { level, acc }) = self.stack.pop() {
                    let text = render_heading(level, &acc);
                    self.push_block(text, BlockTag::Other);
                }
            }
            OwnedEvent::StartBlockquote => {
                self.stack.push(Frame::Blockquote { buf: String::new() });
            }
            OwnedEvent::EndBlockquote => {
                if let Some(Frame::Blockquote { buf }) = self.stack.pop() {
                    let mut text = String::new();
                    for line in buf.lines() {
                        text.push_str("   ");
                        text.push_str(line);
                        text.push('\n');
                    }
                    text.push('\n');
                    self.push_block(text, BlockTag::Other);
                }
            }
            OwnedEvent::StartList { ordered } => {
                self.list_depth += 1;
                self.stack.push(Frame::List {
                    ordered,
                    buf: String::new(),
                });
            }
            OwnedEvent::EndList => {
                if let Some(Frame::List { buf, .. }) = self.stack.pop() {
                    self.list_depth -= 1;
                    // `buf` already holds every item's rendered text,
                    // appended directly as each `EndListItem` completed
                    // (matching `build_list`'s single growing `ctx.output`,
                    // not a collect-then-concatenate pass).
                    let mut text = buf;
                    text.push('\n');
                    self.push_block(text, BlockTag::List);
                }
            }
            OwnedEvent::StartListItem => {
                self.stack.push(Frame::ListItem { children: vec![] });
            }
            OwnedEvent::EndListItem => {
                if let Some(Frame::ListItem { children }) = self.stack.pop() {
                    let ordered =
                        matches!(self.stack.last(), Some(Frame::List { ordered: true, .. }));
                    let text = self.render_list_item(ordered, children);
                    if let Some(Frame::List { buf, .. }) = self.stack.last_mut() {
                        buf.push_str(&text);
                    }
                }
            }
            OwnedEvent::StartCodeBlock { language } => {
                self.stack.push(Frame::CodeBlock {
                    language,
                    content: String::new(),
                });
            }
            OwnedEvent::CodeBlockContent(content) => {
                if let Some(Frame::CodeBlock { content: buf, .. }) = self.stack.last_mut() {
                    buf.push_str(&content);
                }
            }
            OwnedEvent::EndCodeBlock => {
                if let Some(Frame::CodeBlock { language, content }) = self.stack.pop() {
                    let mut text = String::new();
                    if let Some(lang) = &language {
                        text.push_str(".. code-block:: ");
                        text.push_str(lang);
                        text.push_str("\n\n");
                    } else {
                        text.push_str("::\n\n");
                    }
                    for line in content.lines() {
                        text.push_str("   ");
                        text.push_str(line);
                        text.push('\n');
                    }
                    text.push('\n');
                    self.push_block(text, BlockTag::Other);
                }
            }
            OwnedEvent::RawBlock { format, content } => {
                let text = if format == "rst" {
                    content
                } else {
                    String::new()
                };
                self.push_block(text, BlockTag::Other);
            }
            OwnedEvent::StartDiv { .. } => {
                self.stack.push(Frame::Div { buf: String::new() });
            }
            OwnedEvent::EndDiv => {
                if let Some(Frame::Div { buf }) = self.stack.pop() {
                    self.push_block(buf, BlockTag::Other);
                }
            }
            OwnedEvent::HorizontalRule => {
                self.push_block("----\n\n".to_string(), BlockTag::Other);
            }
            OwnedEvent::StartTable => {
                self.stack.push(Frame::Table { rows: vec![] });
            }
            OwnedEvent::EndTable => {
                if let Some(Frame::Table { rows }) = self.stack.pop() {
                    let text = render_table(&rows);
                    self.push_block(text, BlockTag::Other);
                }
            }
            OwnedEvent::StartTableRow { is_header } => {
                self.stack.push(Frame::TableRow {
                    cells: vec![],
                    is_header,
                });
            }
            OwnedEvent::EndTableRow => {
                if let Some(Frame::TableRow { cells, is_header }) = self.stack.pop() {
                    if let Some(Frame::Table { rows }) = self.stack.last_mut() {
                        rows.push((cells, is_header));
                    }
                }
            }
            OwnedEvent::StartTableCell => {
                self.stack.push(Frame::TableCell {
                    plain: String::new(),
                });
            }
            OwnedEvent::EndTableCell => {
                if let Some(Frame::TableCell { plain }) = self.stack.pop() {
                    if let Some(Frame::TableRow { cells, .. }) = self.stack.last_mut() {
                        cells.push(plain);
                    }
                }
            }
            OwnedEvent::StartDefinitionList => {
                self.stack.push(Frame::DefinitionList {
                    buf: String::new(),
                    pending_term: String::new(),
                });
            }
            OwnedEvent::EndDefinitionList => {
                if let Some(Frame::DefinitionList { buf, .. }) = self.stack.pop() {
                    self.push_block(buf, BlockTag::Other);
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                self.stack.push(Frame::DefinitionTerm(String::new()));
            }
            OwnedEvent::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm(term)) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { pending_term, .. }) = self.stack.last_mut()
                    {
                        *pending_term = term;
                    }
                }
            }
            OwnedEvent::StartDefinitionDesc => {
                self.stack.push(Frame::DefinitionDesc(String::new()));
            }
            OwnedEvent::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc(desc)) = self.stack.pop() {
                    if let Some(Frame::DefinitionList { buf, pending_term }) = self.stack.last_mut()
                    {
                        // Each definition list item completes (term, then
                        // desc) in strict sequence, so the term is held only
                        // momentarily and immediately spliced in here — no
                        // `Vec<(String, String)>` of items is ever retained.
                        buf.push_str(pending_term);
                        buf.push('\n');
                        buf.push_str("   ");
                        buf.push_str(&desc);
                        buf.push_str("\n\n");
                        pending_term.clear();
                    }
                }
            }
            OwnedEvent::StartFootnoteDef { label } => {
                self.stack.push(Frame::FootnoteDef {
                    label,
                    buf: String::new(),
                });
            }
            OwnedEvent::EndFootnoteDef => {
                if let Some(Frame::FootnoteDef { label, buf }) = self.stack.pop() {
                    let mut text = String::new();
                    text.push_str(".. [");
                    text.push_str(&label);
                    text.push_str("] ");
                    text.push_str(&buf);
                    text.push('\n');
                    self.push_block(text, BlockTag::Other);
                }
            }
            OwnedEvent::MathDisplay { source } => {
                let mut text = String::new();
                text.push_str(".. math::\n\n   ");
                text.push_str(&source.replace('\n', "\n   "));
                text.push_str("\n\n");
                self.push_block(text, BlockTag::Other);
            }
            OwnedEvent::StartAdmonition { admonition_type } => {
                self.stack.push(Frame::Admonition {
                    admonition_type,
                    buf: String::new(),
                });
            }
            OwnedEvent::EndAdmonition => {
                if let Some(Frame::Admonition {
                    admonition_type,
                    buf,
                }) = self.stack.pop()
                {
                    let mut text = String::new();
                    text.push_str(".. ");
                    text.push_str(&admonition_type);
                    text.push_str("::\n\n");
                    for line in buf.lines() {
                        text.push_str("   ");
                        text.push_str(line);
                        text.push('\n');
                    }
                    text.push('\n');
                    self.push_block(text, BlockTag::Other);
                }
            }
            OwnedEvent::StartFigure { url, alt } => {
                self.stack.push(Frame::Figure {
                    url,
                    alt,
                    caption_buf: String::new(),
                    caption_any: false,
                });
            }
            OwnedEvent::EndFigure => {
                if let Some(Frame::Figure {
                    url,
                    alt,
                    caption_buf,
                    caption_any,
                }) = self.stack.pop()
                {
                    let mut text = String::new();
                    text.push_str(".. figure:: ");
                    text.push_str(&url);
                    text.push('\n');
                    if let Some(alt_text) = &alt {
                        text.push_str("   :alt: ");
                        text.push_str(alt_text);
                        text.push('\n');
                    }
                    if caption_any {
                        text.push_str("\n   ");
                        text.push_str(&caption_buf);
                        text.push('\n');
                    }
                    text.push('\n');
                    self.push_block(text, BlockTag::Other);
                }
            }
            OwnedEvent::ImageBlock { url, alt, .. } => {
                let mut text = String::new();
                text.push_str(".. image:: ");
                text.push_str(&url);
                text.push('\n');
                if let Some(alt_text) = &alt {
                    text.push_str("   :alt: ");
                    text.push_str(alt_text);
                    text.push('\n');
                }
                text.push('\n');
                self.push_block(text, BlockTag::Other);
            }

            // ── Inline events ──────────────────────────────────────────────────
            OwnedEvent::Text(cow) => {
                self.push_inline(&cow, &cow);
            }
            OwnedEvent::SoftBreak => {
                self.push_inline("\n", " ");
            }
            OwnedEvent::LineBreak => {
                self.push_inline("\n", " ");
            }
            OwnedEvent::StartEmphasis => {
                self.stack.push(Frame::Emphasis(InlineAccum::new()));
            }
            OwnedEvent::EndEmphasis => {
                if let Some(Frame::Emphasis(acc)) = self.stack.pop() {
                    self.push_wrapped("*", &acc.buf, "*", &acc.plain);
                }
            }
            OwnedEvent::StartStrong => {
                self.stack.push(Frame::Strong(InlineAccum::new()));
            }
            OwnedEvent::EndStrong => {
                if let Some(Frame::Strong(acc)) = self.stack.pop() {
                    self.push_wrapped("**", &acc.buf, "**", &acc.plain);
                }
            }
            OwnedEvent::StartStrikeout => {
                self.stack.push(Frame::Strikeout(InlineAccum::new()));
            }
            OwnedEvent::EndStrikeout => {
                if let Some(Frame::Strikeout(acc)) = self.stack.pop() {
                    self.push_wrapped(":strike:`", &acc.buf, "`", &acc.plain);
                }
            }
            OwnedEvent::StartUnderline => {
                self.stack.push(Frame::Underline(InlineAccum::new()));
            }
            OwnedEvent::EndUnderline => {
                if let Some(Frame::Underline(acc)) = self.stack.pop() {
                    self.push_wrapped(":underline:`", &acc.buf, "`", &acc.plain);
                }
            }
            OwnedEvent::StartSubscript => {
                self.stack.push(Frame::Subscript(InlineAccum::new()));
            }
            OwnedEvent::EndSubscript => {
                if let Some(Frame::Subscript(acc)) = self.stack.pop() {
                    self.push_wrapped(":sub:`", &acc.buf, "`", &acc.plain);
                }
            }
            OwnedEvent::StartSuperscript => {
                self.stack.push(Frame::Superscript(InlineAccum::new()));
            }
            OwnedEvent::EndSuperscript => {
                if let Some(Frame::Superscript(acc)) = self.stack.pop() {
                    self.push_wrapped(":sup:`", &acc.buf, "`", &acc.plain);
                }
            }
            OwnedEvent::StartSmallCaps => {
                self.stack.push(Frame::SmallCaps(InlineAccum::new()));
            }
            OwnedEvent::EndSmallCaps => {
                if let Some(Frame::SmallCaps(acc)) = self.stack.pop() {
                    self.push_wrapped(":sc:`", &acc.buf, "`", &acc.plain);
                }
            }
            OwnedEvent::Code(cow) => {
                self.push_wrapped("``", &cow, "``", &cow);
            }
            OwnedEvent::StartLink { url } => {
                self.stack.push(Frame::Link {
                    url,
                    acc: InlineAccum::new(),
                });
            }
            OwnedEvent::EndLink => {
                if let Some(Frame::Link { url, acc }) = self.stack.pop() {
                    self.push_multi(&["`", &acc.buf, " <", &url, ">`_"], &acc.plain);
                }
            }
            OwnedEvent::InlineImage { url, alt } => {
                if alt.is_empty() {
                    self.push_multi(&[".. image:: ", &url, "\n"], &alt);
                } else {
                    self.push_multi(&[".. image:: ", &url, "\n   :alt: ", &alt, "\n"], &alt);
                }
            }
            OwnedEvent::FootnoteRef { label } => {
                self.push_wrapped("[", &label, "]_", &label);
            }
            OwnedEvent::StartFootnoteDefInline { label } => {
                self.stack.push(Frame::FootnoteDefInline {
                    label,
                    acc: InlineAccum::new(),
                });
            }
            OwnedEvent::EndFootnoteDefInline => {
                if let Some(Frame::FootnoteDefInline { label, acc }) = self.stack.pop() {
                    self.push_multi(&[".. [", &label, "] ", &acc.buf], &acc.plain);
                }
            }
            OwnedEvent::StartQuoted { quote_type } => {
                self.stack.push(Frame::Quoted {
                    quote_type,
                    acc: InlineAccum::new(),
                });
            }
            OwnedEvent::EndQuoted => {
                if let Some(Frame::Quoted { quote_type, acc }) = self.stack.pop() {
                    let quote = if quote_type == "single" { "'" } else { "\"" };
                    self.push_wrapped(quote, &acc.buf, quote, &acc.plain);
                }
            }
            OwnedEvent::MathInline { source } => {
                self.push_wrapped(":math:`", &source, "`", &source);
            }
            OwnedEvent::StartRstSpan { role } => {
                self.stack.push(Frame::RstSpan {
                    role,
                    acc: InlineAccum::new(),
                });
            }
            OwnedEvent::EndRstSpan => {
                if let Some(Frame::RstSpan { role, acc }) = self.stack.pop() {
                    self.push_multi(&[":", &role, ":`", &acc.buf, "`"], &acc.plain);
                }
            }
        }
    }

    /// Render one list item's final text, replicating `build_list_item`'s
    /// per-child-kind dispatch: the "- "/"#. " prefix precedes the first
    /// child; subsequent `Paragraph` children get a continuation indent;
    /// `NestedList` children get a blank line + indent before their
    /// already-rendered text; everything else is written verbatim.
    /// `self.list_depth` is read here (not passed in) because it is still
    /// valid — we're still inside the enclosing list's Start/End bracket.
    fn render_list_item(&self, ordered: bool, children: Vec<ListChild>) -> String {
        let mut text = String::new();
        if ordered {
            text.push_str("#. ");
        } else {
            text.push_str("- ");
        }
        let mut first = true;
        for child in children {
            match child {
                ListChild::Paragraph(buf) => {
                    if !first {
                        for _ in 0..self.list_depth {
                            text.push_str("   ");
                        }
                        text.push_str("   ");
                    }
                    text.push_str(&buf);
                    text.push('\n');
                }
                ListChild::NestedList(rendered) => {
                    text.push('\n');
                    for _ in 0..self.list_depth {
                        text.push_str("   ");
                    }
                    text.push_str(&rendered);
                }
                ListChild::Other(rendered) => {
                    text.push_str(&rendered);
                }
            }
            first = false;
        }
        text
    }
}

/// Mirrors `build_heading`: underline length (and overline, for level 1) is
/// computed from the *plain* text's byte length, matching
/// `collect_text_from_inlines`, not the formatted buffer's length (which
/// would wrongly include markup delimiters).
fn render_heading(level: i64, acc: &InlineAccum) -> String {
    let underline_char = match level {
        1 => '=',
        2 => '-',
        3 => '~',
        4 => '^',
        5 => '"',
        _ => '\'',
    };
    let mut text = String::new();
    if level == 1 {
        let line: String = std::iter::repeat_n(underline_char, acc.plain.len()).collect();
        text.push_str(&line);
        text.push('\n');
    }
    text.push_str(&acc.buf);
    text.push('\n');
    let line: String = std::iter::repeat_n(underline_char, acc.plain.len()).collect();
    text.push_str(&line);
    text.push_str("\n\n");
    text
}

/// Mirrors `build_table`: cells only ever carry plain text (never formatted
/// markup), so `rows` is `(cell plain text, is_header)` pairs directly —
/// no `text_rows` recomputation needed since the cells were never anything
/// but plain text to begin with.
fn render_table(rows: &[(Vec<String>, bool)]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let text_rows: Vec<Vec<String>> = rows.iter().map(|(cells, _)| cells.clone()).collect();
    let col_widths = calculate_column_widths(&text_rows);

    let mut ctx = BuildContext::new();
    emit_table_border(&col_widths, &mut ctx);

    let mut is_first = true;
    for (row, text_row) in rows.iter().zip(text_rows.iter()) {
        let (_, is_header) = row;
        ctx.output.push('|');
        for (i, cell) in text_row.iter().enumerate() {
            let width = col_widths.get(i).copied().unwrap_or(1);
            ctx.output.push(' ');
            ctx.output.push_str(cell);
            for _ in cell.len()..width {
                ctx.output.push(' ');
            }
            ctx.output.push_str(" |");
        }
        ctx.output.push('\n');

        if is_first && *is_header && rows.len() > 1 {
            emit_table_border(&col_widths, &mut ctx);
        }
        is_first = false;
    }

    emit_table_border(&col_widths, &mut ctx);
    ctx.output.push('\n');
    ctx.output
}

enum Frame {
    /// A paragraph's formatted buffer. No `plain` tracking — nothing ever
    /// reads a paragraph's plain text back out (see [`Dest::BufOnly`]).
    Paragraph(String),
    Heading {
        level: i64,
        acc: InlineAccum,
    },
    Blockquote {
        buf: String,
    },
    List {
        ordered: bool,
        /// Rendered item texts appended directly as each item completes —
        /// see the comment at `EndList`.
        buf: String,
    },
    ListItem {
        children: Vec<ListChild>,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    Div {
        buf: String,
    },
    Table {
        rows: Vec<(Vec<String>, bool)>,
    },
    TableRow {
        cells: Vec<String>,
        is_header: bool,
    },
    TableCell {
        plain: String,
    },
    DefinitionList {
        /// Rendered `term\n   desc\n\n` text, appended directly as each item
        /// completes (see `EndDefinitionDesc`).
        buf: String,
        /// Holds the most recently closed term until its matching
        /// description arrives.
        pending_term: String,
    },
    /// Term/description buffers — `plain` not tracked, see [`Frame::Paragraph`].
    DefinitionTerm(String),
    DefinitionDesc(String),
    FootnoteDef {
        label: String,
        buf: String,
    },
    Figure {
        url: String,
        alt: Option<String>,
        /// Formatted caption text and whether any caption content was seen
        /// at all (`build_figure` distinguishes "no caption" from "caption
        /// content that happens to be empty" — a plain-text length can't
        /// tell those apart, hence the separate flag rather than `InlineAccum`).
        caption_buf: String,
        caption_any: bool,
    },
    Admonition {
        admonition_type: String,
        buf: String,
    },
    // Inline spans
    Emphasis(InlineAccum),
    Strong(InlineAccum),
    Strikeout(InlineAccum),
    Underline(InlineAccum),
    Subscript(InlineAccum),
    Superscript(InlineAccum),
    SmallCaps(InlineAccum),
    Link {
        url: String,
        acc: InlineAccum,
    },
    FootnoteDefInline {
        label: String,
        acc: InlineAccum,
    },
    Quoted {
        quote_type: String,
        acc: InlineAccum,
    },
    RstSpan {
        role: String,
        acc: InlineAccum,
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
        assert!(s.contains("Hello"), "got: {s:?}");
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
        let input =
            "Section\n=======\n\nA paragraph with *emphasis* text.\n\n- item one\n- item two\n";
        let doc = crate::parse(input).unwrap();
        let evts: Vec<_> = crate::EventIter::new(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let doc2 = crate::parse(&emitted_text).unwrap();
        // Compare block counts as a basic sanity check
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }

    /// Round-trip a broader construct mix (headings, lists, code blocks,
    /// tables, definition lists, footnotes, admonitions, block quotes)
    /// entirely through `events() -> Writer`, proving the incremental
    /// per-top-level-block flush in `Writer::push_block` handles every
    /// construct `parse()` produces, not just paragraphs/headings/lists.
    #[test]
    fn test_writer_roundtrip_full_construct_mix() {
        let input = "\
Title
=====

Intro paragraph with **strong** and ``code``.

    An indented block quote.

- bullet one
- bullet two

1. ordered one
2. ordered two

.. code-block:: rust

   let x = 1;

term
    definition body

+--------+--------+
| A      | B      |
+========+========+
| Cell 1 | Cell 2 |
+--------+--------+

See [1]_ for details.

.. [1] Footnote body text.

----

After the transition.
";
        let doc = crate::parse(input).unwrap();
        assert!(
            doc.blocks.len() >= 10,
            "expected a rich construct mix, got {:?}",
            doc.blocks
        );

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::EventIter::new(input) {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();

        // Re-parsing the writer's output must recover the same block shapes,
        // in the same order — the same discriminant-only comparison
        // `test_events_matches_parse_shape` uses in lib.rs, applied here to
        // prove round-trip parity through the streaming writer specifically.
        let doc2 = crate::parse(&emitted_text).unwrap();
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "writer roundtrip block count mismatch\ninput blocks: {:#?}\nemitted text: {emitted_text}\nreparsed blocks: {:#?}",
            doc.blocks,
            doc2.blocks,
        );
        for (a, b) in doc.blocks.iter().zip(doc2.blocks.iter()) {
            assert_eq!(
                std::mem::discriminant(a),
                std::mem::discriminant(b),
                "block kind mismatch: {a:?} vs {b:?}"
            );
        }
    }

    /// Nested lists (bullet-in-bullet, ordered-in-bullet) are the trickiest
    /// path in the streaming rewrite: `list_depth` bookkeeping happens on
    /// `Writer` itself (bracketing `StartList`/`EndList`), and each nested
    /// list's rendered text is captured as an already-rendered `ListChild`
    /// entry rather than re-rendered from a reconstructed tree.
    #[test]
    fn test_writer_roundtrip_nested_lists() {
        let input = "\
- outer one
- outer two

  - inner a
  - inner b

- outer three

  #. inner ordered one
  #. inner ordered two
";
        let doc = crate::parse(input).unwrap();

        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::EventIter::new(input) {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();

        let doc2 = crate::parse(&emitted_text).unwrap();
        assert_eq!(
            doc.blocks.len(),
            doc2.blocks.len(),
            "nested list roundtrip block count mismatch\nemitted:\n{emitted_text}"
        );
        for (a, b) in doc.blocks.iter().zip(doc2.blocks.iter()) {
            assert_eq!(
                std::mem::discriminant(a),
                std::mem::discriminant(b),
                "block kind mismatch: {a:?} vs {b:?}\nemitted:\n{emitted_text}"
            );
        }
        // The nested lists must actually still be nested after roundtrip,
        // not flattened to sibling top-level lists.
        fn count_lists(blocks: &[crate::Block]) -> usize {
            let mut n = 0;
            for b in blocks {
                if let crate::Block::List { items, .. } = b {
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
            count_lists(&doc2.blocks) >= 3,
            "expected outer list + 2 nested lists, emitted:\n{emitted_text}"
        );
    }

    /// Regression guard against reintroducing per-block `Block`/`Inline`
    /// subtree reconstruction (the original perf bug: `Writer::process`
    /// rebuilt an owned tree per top-level block, then ran the same
    /// `build_block` pass `build()` already runs directly — ~7-8x slower).
    /// A large, deeply-nested event stream must complete with an allocation
    /// count that stays close to linear in event count, not blow up the way
    /// tree materialization (an `Inline`/`Block` enum + `Vec` per node, on
    /// top of the formatting pass) would. This does not merely check output
    /// correctness — the existing roundtrip tests already do that — it
    /// checks the *cost shape* of getting there.
    #[test]
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
                evs.push(OwnedEvent::StartStrong);
                evs.push(OwnedEvent::Text(Cow::Owned("bold".to_string())));
                evs.push(OwnedEvent::EndStrong);
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
}
