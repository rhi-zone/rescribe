//! Streaming Textile writer — converts a stream of [`TextileEvent`]s to Textile text.
//!
//! Construct classification:
//!
//! - **Write-straight-through**: `Paragraph`'s own `p[attrs][align]. `
//!   prefix and `Heading`'s `h{level}[attrs]. ` prefix (both fully known at
//!   `Start*`), `CodeBlock`/`RawBlock`/`RawInline`/`InlineImage`/
//!   `FootnoteRef`/`Acronym` (single self-contained events), table row/cell
//!   delimiters (attrs/`is_header`/`align` all known at `Start*` — no
//!   column-width computation, Textile's table syntax is per-cell
//!   delimited), `FootnoteDef`'s `fn{label}. ` prefix, `DefinitionList`'s
//!   `;term\n:desc\n` markers, and every inline span with a fixed delimiter
//!   (`Bold`, `Italic`, `Underline`, `Strikethrough`, `Superscript`,
//!   `Subscript`, `Code`, `Citation`, `GenericSpan` — its `%attrs` prefix is
//!   known at `StartGenericSpan`).
//! - **Genuinely deferred, O(1) parent-frame lookup, not a buffer**:
//!   `Paragraph`'s prefix/terminator depends on which frame encloses it —
//!   mirroring `emit::emit_block`'s per-container match arms, which (a
//!   notable quirk replicated exactly, not "fixed") silently ignore a
//!   `Paragraph`'s *own* `align`/`attrs` fields whenever it is a direct
//!   child of `Blockquote` or a list item, using the blockquote's cached
//!   prefix or no prefix at all instead. Likewise `List`'s marker run-length
//!   depends on nesting depth (a persistent `list_depth` counter, mirroring
//!   `EmitContext::list_depth`) and a `List` directly nested inside a list
//!   item needs one extra leading `"\n"` — both O(1) lookups against
//!   already-tracked state, not lookahead.
//! - **Genuinely deferred, O(1) small fixed strings**: `Blockquote`'s
//!   `"bq[attrs]. "` prefix is attrs-dependent but attrs are fixed at
//!   `StartBlockquote`, so it's computed once and cached (bounded by the
//!   attrs' own size, not document size) for reuse by every paragraph
//!   inside. `Link`'s `"title":url` suffix must be written *after* its
//!   children (`emit_inline`'s `Inline::Link` writes children before
//!   title/url) even though both are already known at `StartLink`, so they
//!   wait in the frame until `EndLink` — two small strings, not a buffer.
//!
//! # Example
//! ```no_run
//! use textile_fmt::writer::Writer;
//! use textile_fmt::events::TextileEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(TextileEvent::StartHeading { level: 1, attrs: Default::default() });
//! w.write_event(TextileEvent::Text("Hello".to_string()));
//! w.write_event(TextileEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use std::io::Write;

use crate::ast::BlockAttrs;
use crate::events::TextileEvent;

/// Streaming Textile writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// construct is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to flush any remainder and recover the sink.
pub struct Writer<W: Write> {
    sink: W,
    /// Shared output buffer. Cleared (capacity retained) after each
    /// top-level block is flushed.
    out: String,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level — closing a construct with an empty
    /// stack flushes.
    stack: Vec<Frame>,
    /// Mirrors `EmitContext::list_depth`: incremented on `StartList`,
    /// decremented on `EndList`.
    list_depth: usize,
}

enum Frame {
    Paragraph,
    Heading,
    /// Cached `"bq[attrs]. "` prefix, computed once from `StartBlockquote`'s
    /// attrs and reused for each direct `Paragraph` child.
    Blockquote {
        prefix: String,
    },
    List {
        ordered: bool,
    },
    ListItem,
    Table,
    TableRow,
    TableCell,
    FootnoteDef,
    DefinitionList,
    DefinitionTerm,
    DefinitionDesc,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Superscript,
    Subscript,
    /// `url`/`title` are known at `StartLink` but must be written *after*
    /// the link's children — see the module doc.
    Link {
        url: String,
        title: Option<String>,
    },
    Citation,
    GenericSpan,
}

const DEFAULT_OUT_CAPACITY: usize = 4096;

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Self::with_capacity(sink, DEFAULT_OUT_CAPACITY)
    }

    /// Like [`Writer::new`], but reserves `out_capacity` bytes up front.
    pub fn with_capacity(sink: W, out_capacity: usize) -> Self {
        Writer {
            sink,
            out: String::with_capacity(out_capacity),
            stack: Vec::new(),
            list_depth: 0,
        }
    }

    /// Feed one event to the writer. Writes bytes to the sink immediately
    /// whenever this event completes a top-level construct.
    pub fn write_event(&mut self, event: TextileEvent) {
        self.process(event);
        self.maybe_flush();
    }

    /// Flush any remaining buffered bytes and recover the sink.
    pub fn finish(mut self) -> W {
        self.flush();
        self.sink
    }

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

    fn push(&mut self, s: &str) {
        self.out.push_str(s);
    }

    /// Whether the current innermost frame accepts inline text/markup.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph
                    | Frame::Heading
                    | Frame::FootnoteDef
                    | Frame::DefinitionTerm
                    | Frame::DefinitionDesc
                    | Frame::TableCell
                    | Frame::Bold
                    | Frame::Italic
                    | Frame::Underline
                    | Frame::Strikethrough
                    | Frame::Superscript
                    | Frame::Subscript
                    | Frame::Link { .. }
                    | Frame::Citation
                    | Frame::GenericSpan
            )
        )
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: TextileEvent) {
        match event {
            // ── Paragraph: prefix/terminator depend on the enclosing frame ──
            TextileEvent::StartParagraph { align, attrs } => {
                match self.stack.last() {
                    Some(Frame::Blockquote { prefix }) => {
                        let prefix = prefix.clone();
                        self.push(&prefix);
                    }
                    Some(Frame::ListItem) => {}
                    _ => {
                        let has_align = align.is_some();
                        let has_attrs = !attrs.is_empty();
                        if has_align || has_attrs {
                            self.push("p");
                            let attrs_str = format_block_attrs(&attrs);
                            self.push(&attrs_str);
                            if let Some(a) = &align {
                                self.push(align_marker(a));
                            }
                            self.push(". ");
                        }
                    }
                }
                self.stack.push(Frame::Paragraph);
            }
            TextileEvent::EndParagraph => {
                if matches!(self.stack.last(), Some(Frame::Paragraph)) {
                    self.stack.pop();
                    let term = match self.stack.last() {
                        Some(Frame::ListItem) => "",
                        _ => "\n\n",
                    };
                    self.push(term);
                }
            }

            TextileEvent::StartHeading { level, attrs } => {
                self.push(&format!("h{level}"));
                let attrs_str = format_block_attrs(&attrs);
                self.push(&attrs_str);
                self.push(". ");
                self.stack.push(Frame::Heading);
            }
            TextileEvent::EndHeading => {
                if matches!(self.stack.last(), Some(Frame::Heading)) {
                    self.stack.pop();
                    self.push("\n\n");
                }
            }

            TextileEvent::CodeBlock { content, language } => {
                if let Some(lang) = &language {
                    self.push(&format!("bc({lang}). "));
                } else {
                    self.push("bc. ");
                }
                self.push(&content);
                self.push("\n\n");
            }

            TextileEvent::StartBlockquote { attrs } => {
                let prefix = if !attrs.is_empty() {
                    let mut p = String::from("bq");
                    p.push_str(&format_block_attrs(&attrs));
                    p.push_str(". ");
                    p
                } else {
                    "bq. ".to_string()
                };
                self.stack.push(Frame::Blockquote { prefix });
            }
            TextileEvent::EndBlockquote => {
                if matches!(self.stack.last(), Some(Frame::Blockquote { .. })) {
                    self.stack.pop();
                }
            }

            TextileEvent::StartList { ordered } => {
                if matches!(self.stack.last(), Some(Frame::ListItem)) {
                    self.push("\n");
                }
                self.list_depth += 1;
                self.stack.push(Frame::List { ordered });
            }
            TextileEvent::EndList => {
                if matches!(self.stack.last(), Some(Frame::List { .. })) {
                    self.stack.pop();
                    self.list_depth -= 1;
                    if self.list_depth == 0 {
                        self.push("\n");
                    }
                }
            }
            TextileEvent::StartListItem => {
                if let Some(Frame::List { ordered }) = self.stack.last() {
                    let marker = if *ordered { '#' } else { '*' };
                    for _ in 0..self.list_depth {
                        self.out.push(marker);
                    }
                    self.push(" ");
                }
                self.stack.push(Frame::ListItem);
            }
            TextileEvent::EndListItem => {
                if matches!(self.stack.last(), Some(Frame::ListItem)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }

            TextileEvent::StartTable => self.stack.push(Frame::Table),
            TextileEvent::EndTable => {
                if matches!(self.stack.last(), Some(Frame::Table)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }
            TextileEvent::StartTableRow { attrs } => {
                if !attrs.is_empty() {
                    let attrs_str = format_block_attrs(&attrs);
                    self.push(&attrs_str);
                    self.push(". ");
                }
                self.stack.push(Frame::TableRow);
            }
            TextileEvent::EndTableRow => {
                if matches!(self.stack.last(), Some(Frame::TableRow)) {
                    self.stack.pop();
                    self.push("|\n");
                }
            }
            TextileEvent::StartTableCell { is_header, align } => {
                self.push("|");
                let align_str = align.as_deref().map(cell_align_marker);
                if is_header {
                    self.push("_");
                    match align_str.filter(|s| !s.is_empty()) {
                        Some(a) => self.push(a),
                        None => self.push("."),
                    }
                    self.push(" ");
                } else if let Some(a) = align_str.filter(|s| !s.is_empty()) {
                    self.push(a);
                    self.push(" ");
                }
                self.stack.push(Frame::TableCell);
            }
            TextileEvent::EndTableCell => {
                if matches!(self.stack.last(), Some(Frame::TableCell)) {
                    self.stack.pop();
                }
            }

            TextileEvent::HorizontalRule => self.push("---\n\n"),

            TextileEvent::StartFootnoteDef { label } => {
                self.push(&format!("fn{label}. "));
                self.stack.push(Frame::FootnoteDef);
            }
            TextileEvent::EndFootnoteDef => {
                if matches!(self.stack.last(), Some(Frame::FootnoteDef)) {
                    self.stack.pop();
                    self.push("\n\n");
                }
            }

            TextileEvent::StartDefinitionList => self.stack.push(Frame::DefinitionList),
            TextileEvent::EndDefinitionList => {
                if matches!(self.stack.last(), Some(Frame::DefinitionList)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }
            TextileEvent::StartDefinitionTerm => {
                self.push(";");
                self.stack.push(Frame::DefinitionTerm);
            }
            TextileEvent::EndDefinitionTerm => {
                if matches!(self.stack.last(), Some(Frame::DefinitionTerm)) {
                    self.stack.pop();
                    self.push("\n:");
                }
            }
            TextileEvent::StartDefinitionDesc => self.stack.push(Frame::DefinitionDesc),
            TextileEvent::EndDefinitionDesc => {
                if matches!(self.stack.last(), Some(Frame::DefinitionDesc)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }

            TextileEvent::RawBlock { content } => {
                self.push("notextile. ");
                self.push(&content);
                self.push("\n\n");
            }

            // ── Inline span open/close ──────────────────────────────────────
            TextileEvent::StartBold => self.write_delim_open("*", Frame::Bold),
            TextileEvent::EndBold => self.write_delim_close(is_bold, "*"),
            TextileEvent::StartItalic => self.write_delim_open("_", Frame::Italic),
            TextileEvent::EndItalic => self.write_delim_close(is_italic, "_"),
            TextileEvent::StartUnderline => self.write_delim_open("+", Frame::Underline),
            TextileEvent::EndUnderline => self.write_delim_close(is_underline, "+"),
            TextileEvent::StartStrikethrough => self.write_delim_open("-", Frame::Strikethrough),
            TextileEvent::EndStrikethrough => self.write_delim_close(is_strikethrough, "-"),
            TextileEvent::StartSuperscript => self.write_delim_open("^", Frame::Superscript),
            TextileEvent::EndSuperscript => self.write_delim_close(is_superscript, "^"),
            TextileEvent::StartSubscript => self.write_delim_open("~", Frame::Subscript),
            TextileEvent::EndSubscript => self.write_delim_close(is_subscript, "~"),
            TextileEvent::StartCitation => self.write_delim_open("??", Frame::Citation),
            TextileEvent::EndCitation => self.write_delim_close(is_citation, "??"),

            TextileEvent::StartLink { url, title } => {
                if self.accepts_inline() {
                    self.push("\"");
                }
                self.stack.push(Frame::Link { url, title });
            }
            TextileEvent::EndLink => {
                if let Some(Frame::Link { url, title }) = self.stack.pop()
                    && self.accepts_inline()
                {
                    if let Some(t) = &title {
                        self.push("(");
                        self.push(t);
                        self.push(")");
                    }
                    self.push("\":");
                    self.push(&url);
                }
            }

            TextileEvent::StartGenericSpan { attrs } => {
                if self.accepts_inline() {
                    self.push("%");
                    if !attrs.is_empty() {
                        let attrs_str = format_inline_attrs(&attrs);
                        self.push(&attrs_str);
                    }
                }
                self.stack.push(Frame::GenericSpan);
            }
            TextileEvent::EndGenericSpan => {
                if matches!(self.stack.last(), Some(Frame::GenericSpan)) {
                    self.stack.pop();
                    if self.accepts_inline() {
                        self.push("%");
                    }
                }
            }

            // ── Leaf inlines ────────────────────────────────────────────────
            TextileEvent::Text(s) => {
                if self.accepts_inline() {
                    self.push(&s);
                }
            }
            TextileEvent::InlineCode(s) => {
                if self.accepts_inline() {
                    self.push("@");
                    self.push(&s);
                    self.push("@");
                }
            }
            TextileEvent::InlineImage { url, alt } => {
                if self.accepts_inline() {
                    self.push("!");
                    self.push(&url);
                    if let Some(a) = &alt {
                        self.push("(");
                        self.push(a);
                        self.push(")");
                    }
                    self.push("!");
                }
            }
            TextileEvent::FootnoteRef { label } => {
                if self.accepts_inline() {
                    self.push("[");
                    self.push(&label);
                    self.push("]");
                }
            }
            TextileEvent::LineBreak => {
                if self.accepts_inline() {
                    self.push("\n");
                }
            }
            TextileEvent::RawInline { content } => {
                if self.accepts_inline() {
                    self.push("==");
                    self.push(&content);
                    self.push("==");
                }
            }
            TextileEvent::Acronym { text, title } => {
                if self.accepts_inline() {
                    self.push(&text);
                    self.push("(");
                    self.push(&title);
                    self.push(")");
                }
            }
        }
    }

    fn write_delim_open(&mut self, delim: &str, frame: Frame) {
        if self.accepts_inline() {
            self.push(delim);
        }
        self.stack.push(frame);
    }

    fn write_delim_close(&mut self, is_expected: fn(&Frame) -> bool, delim: &str) {
        let matches_top = self.stack.last().is_some_and(is_expected);
        if matches_top {
            self.stack.pop();
            // The closing delimiter belongs to the span that just closed,
            // mirroring `write_delim_open`'s unconditional emission.
            self.push(delim);
        }
    }
}

fn is_bold(f: &Frame) -> bool {
    matches!(f, Frame::Bold)
}
fn is_italic(f: &Frame) -> bool {
    matches!(f, Frame::Italic)
}
fn is_underline(f: &Frame) -> bool {
    matches!(f, Frame::Underline)
}
fn is_strikethrough(f: &Frame) -> bool {
    matches!(f, Frame::Strikethrough)
}
fn is_superscript(f: &Frame) -> bool {
    matches!(f, Frame::Superscript)
}
fn is_subscript(f: &Frame) -> bool {
    matches!(f, Frame::Subscript)
}
fn is_citation(f: &Frame) -> bool {
    matches!(f, Frame::Citation)
}

fn align_marker(a: &str) -> &'static str {
    match a {
        "left" => "<",
        "right" => ">",
        "center" => "=",
        "justify" => "<>",
        _ => "",
    }
}

fn cell_align_marker(a: &str) -> &'static str {
    match a {
        "left" => "<.",
        "right" => ">.",
        "center" => "=.",
        "justify" => "<>.",
        _ => "",
    }
}

/// Mirrors `emit::emit_block_attrs`: `(class#id){style}[lang]((...))`.
fn format_block_attrs(attrs: &BlockAttrs) -> String {
    let mut s = String::new();
    if let Some(class) = &attrs.class {
        s.push('(');
        s.push_str(class);
        if let Some(id) = &attrs.id {
            s.push('#');
            s.push_str(id);
        }
        s.push(')');
    } else if let Some(id) = &attrs.id {
        s.push_str("(#");
        s.push_str(id);
        s.push(')');
    }
    if let Some(style) = &attrs.style {
        s.push('{');
        s.push_str(style);
        s.push('}');
    }
    if let Some(lang) = &attrs.lang {
        s.push('[');
        s.push_str(lang);
        s.push(']');
    }
    for _ in 0..attrs.indent_left {
        s.push('(');
    }
    for _ in 0..attrs.indent_right {
        s.push(')');
    }
    s
}

/// Mirrors `emit::emit_inline_attrs`: `{style}(class#id)[lang]` — note the
/// different field order from `format_block_attrs`.
fn format_inline_attrs(attrs: &BlockAttrs) -> String {
    let mut s = String::new();
    if let Some(style) = &attrs.style {
        s.push('{');
        s.push_str(style);
        s.push('}');
    }
    if let Some(class) = &attrs.class {
        s.push('(');
        s.push_str(class);
        if let Some(id) = &attrs.id {
            s.push('#');
            s.push_str(id);
        }
        s.push(')');
    } else if let Some(id) = &attrs.id {
        s.push_str("(#");
        s.push_str(id);
        s.push(')');
    }
    if let Some(lang) = &attrs.lang {
        s.push('[');
        s.push_str(lang);
        s.push(']');
    }
    s
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::BlockAttrs;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartHeading {
            level: 1,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::Text("Hello".to_string()));
        w.write_event(TextileEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("h1. Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::Text("World".to_string()));
        w.write_event(TextileEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_bold() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::StartBold);
        w.write_event(TextileEvent::Text("bold".to_string()));
        w.write_event(TextileEvent::EndBold);
        w.write_event(TextileEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("*bold*"), "got: {s:?}");
    }

    #[test]
    fn test_writer_code_block() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::CodeBlock {
            content: "print('hi')".to_string(),
            language: None,
        });
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("bc. print('hi')"), "got: {s:?}");
    }

    #[test]
    fn test_writer_footnote_def() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartFootnoteDef {
            label: "1".to_string(),
        });
        w.write_event(TextileEvent::Text("Note content".to_string()));
        w.write_event(TextileEvent::EndFootnoteDef);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("fn1. Note content"), "got: {s:?}");
    }

    #[test]
    fn test_writer_definition_list() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartDefinitionList);
        w.write_event(TextileEvent::StartDefinitionTerm);
        w.write_event(TextileEvent::Text("Term".to_string()));
        w.write_event(TextileEvent::EndDefinitionTerm);
        w.write_event(TextileEvent::StartDefinitionDesc);
        w.write_event(TextileEvent::Text("Definition".to_string()));
        w.write_event(TextileEvent::EndDefinitionDesc);
        w.write_event(TextileEvent::EndDefinitionList);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains(";Term"), "got: {s:?}");
        assert!(s.contains(":Definition"), "got: {s:?}");
    }

    #[test]
    fn test_writer_horizontal_rule() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::HorizontalRule);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("---"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "h1. Title\n\nA paragraph with *bold* text.\n\n* item one\n* item two\n";
        let evts: Vec<_> = crate::events::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        // Verify the emitted text re-parses cleanly
        let (doc_orig, _) = crate::parse::parse(input);
        let (doc_emit, _) = crate::parse::parse(&emitted_text);
        assert_eq!(
            doc_orig.blocks.len(),
            doc_emit.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }

    #[test]
    fn test_writer_citation() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::StartCitation);
        w.write_event(TextileEvent::Text("cited".to_string()));
        w.write_event(TextileEvent::EndCitation);
        w.write_event(TextileEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("??cited??"), "got: {s:?}");
    }

    #[test]
    fn test_writer_acronym() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::Acronym {
            text: "HTML".to_string(),
            title: "HyperText Markup Language".to_string(),
        });
        w.write_event(TextileEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("HTML(HyperText Markup Language)"), "got: {s:?}");
    }

    #[test]
    fn test_writer_raw_block() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::RawBlock {
            content: "<b>raw</b>".to_string(),
        });
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("notextile. <b>raw</b>"), "got: {s:?}");
    }

    #[test]
    fn test_writer_raw_inline() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: BlockAttrs::default(),
        });
        w.write_event(TextileEvent::RawInline {
            content: "<em>x</em>".to_string(),
        });
        w.write_event(TextileEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("==<em>x</em>=="), "got: {s:?}");
    }

    /// The streaming `Writer` must produce byte-identical output to the
    /// tree-based `emit::emit` for the same document — including the
    /// quirky context-dependent `Paragraph` handling inside `Blockquote`/
    /// list items, nested lists, tables, definition lists, and attrs-bearing
    /// constructs.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "h1. Title\n\nA paragraph.\n",
            "p(class#id){color:red}[en]<. Aligned styled paragraph.\n",
            "h2(myclass). Styled heading\n\nBody.\n",
            "A paragraph with *bold*, _italic_, -strike-, +underline+, ^super^, ~sub~, @code@, \"a link\":http://x/, \"titled link\"(a title):http://x/, [1], !img.png(alt text)!, ??cited??, %styled span%, %{color:red}(cls#id)[en]attrs span%, HTML(HyperText Markup Language).\n",
            "* item one\n* item two\n\n# ordered one\n# ordered two\n",
            "* outer\n** inner a\n** inner b\n* outer two\n",
            "bq. A quoted paragraph.\n\nSecond paragraph in quote.\n",
            "bq(myclass). Styled blockquote.\n",
            "|_. Name|_. Age|\n|Alice|30|\n",
            "|<. left|>. right|=. center|\n",
            "table(mytable).\n|a|b|\n",
            "term1\n:definition one\n\nterm2\n:definition two\n",
            "bc. fn main() {}\n",
            "bc(rust). fn main() {}\n",
            "notextile. <b>raw</b>\n",
            "---\n",
            "fn1. A footnote body.\n",
            "* item with **paragraph** text\n\np. continued content\n\n* second item\n",
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
                "streaming Writer diverged from build() for input:\n{input}\n\
                 build():\n{built:?}\nWriter:\n{streamed:?}"
            );
        }
    }

    /// Regression guard against reintroducing per-event `Block`/`Inline`
    /// subtree reconstruction, plus a peak-memory-stays-roughly-constant
    /// guard (both as ratios between a small and large run rather than
    /// absolute thresholds — this test binary's threads share one
    /// process-wide `#[global_allocator]`, so an absolute threshold isn't
    /// robust to unrelated concurrent tests' allocation noise, but a
    /// genuine unbounded-growth bug still shows up as a large ratio between
    /// the two runs).
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct TrackingAlloc;
        static ALLOCS: AtomicUsize = AtomicUsize::new(0);
        static CURRENT_BYTES: AtomicUsize = AtomicUsize::new(0);
        static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);
        unsafe impl GlobalAlloc for TrackingAlloc {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                ALLOCS.fetch_add(1, Ordering::Relaxed);
                let cur = CURRENT_BYTES.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
                PEAK_BYTES.fetch_max(cur, Ordering::SeqCst);
                unsafe { System.alloc(layout) }
            }
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                CURRENT_BYTES.fetch_sub(layout.size(), Ordering::SeqCst);
                unsafe { System.dealloc(ptr, layout) }
            }
        }
        #[global_allocator]
        static GLOBAL: TrackingAlloc = TrackingAlloc;

        fn events_for(n: usize) -> Vec<TextileEvent> {
            let mut evs = Vec::new();
            for i in 0..n {
                evs.push(TextileEvent::StartHeading {
                    level: 2,
                    attrs: BlockAttrs::default(),
                });
                evs.push(TextileEvent::Text(format!("Section {i}")));
                evs.push(TextileEvent::EndHeading);
                evs.push(TextileEvent::StartParagraph {
                    align: None,
                    attrs: BlockAttrs::default(),
                });
                evs.push(TextileEvent::Text("plain ".to_string()));
                evs.push(TextileEvent::StartBold);
                evs.push(TextileEvent::Text("bold".to_string()));
                evs.push(TextileEvent::EndBold);
                evs.push(TextileEvent::EndParagraph);
                evs.push(TextileEvent::StartList { ordered: false });
                for j in 0..2 {
                    evs.push(TextileEvent::StartListItem);
                    evs.push(TextileEvent::Text(format!("item {j}")));
                    evs.push(TextileEvent::EndListItem);
                }
                evs.push(TextileEvent::EndList);
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
            "allocation count did not scale near-linearly: {small} allocs @200 -> \
             {large} allocs @2000 (ratio {ratio:.2}); this suggests reintroduced \
             per-block subtree reconstruction"
        );

        fn run_peak(n: usize) -> usize {
            PEAK_BYTES.store(0, Ordering::SeqCst);
            let before = CURRENT_BYTES.load(Ordering::SeqCst);
            let evs = events_for(n);
            let mut out = Vec::new();
            {
                let mut w = Writer::new(&mut out);
                for e in evs {
                    w.write_event(e);
                }
                w.finish();
            }
            std::hint::black_box(&out);
            PEAK_BYTES.load(Ordering::SeqCst).saturating_sub(before)
        }

        let small_peak = run_peak(500).max(1);
        let large_peak = run_peak(5000);
        let peak_ratio = large_peak as f64 / small_peak as f64;
        assert!(
            peak_ratio < 20.0,
            "peak memory did not stay roughly constant across document sizes: \
             {small_peak} bytes peak @500 -> {large_peak} bytes peak @5000 \
             (ratio {peak_ratio:.2}); this suggests the writer is buffering O(document) \
             instead of O(nesting depth)"
        );
    }
}
