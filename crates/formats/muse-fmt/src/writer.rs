//! Streaming Muse writer — converts a stream of events to Muse text.
//!
//! Construct classification:
//!
//! - **Write-straight-through**: every block/inline wrapper whose markup is
//!   fixed text known at `Start*` (headings' `*` run, `<quote>`/`<verse>`/
//!   `<center>`/`<right>` wrappers, list-item ` - `/` N. ` prefixes, table
//!   row/cell delimiters, definition-list ` :: ` separator, footnote `[label]
//!   `, all inline spans, `Link` (its url is known at `StartLink`, and
//!   unlike pod-fmt/haddock-fmt's link, Muse's `[[url][children]]` genuinely
//!   renders its children — but the url still doesn't need to wait for
//!   them)) and every self-contained leaf event (`CodeBlock`, `SrcBlock`,
//!   `LiteralBlock`, `Comment`, `HorizontalRule`, `FootnoteRef`, `LineBreak`,
//!   `Anchor`, `Image`, `Code`) — content already fully in hand.
//! - **Genuinely deferred, O(1) parent lookup, not a buffer**: `Paragraph`'s
//!   closing separator depends on *which frame encloses it* — `"\n\n"` at
//!   document top level, a single `"\n"` inside `Blockquote`/`Verse`/
//!   `CenteredBlock`/`RightBlock`, or nothing at all inside `ListItem`/
//!   `DefinitionDesc` (whose own `"\n"` comes once, after all of the item's
//!   blocks close) — mirroring `emit::build_block`'s per-container match
//!   arms exactly. This is answered by peeking at the frame stack's new top
//!   after popping `Paragraph`, not by buffering anything.
//! - **Genuinely deferred, O(metadata field count)**: the new `Metadata`
//!   event (see `events.rs`) carries the five `#title`/`#author`/`#date`/
//!   `#desc`/`#keywords` directive values, but `emit::build`'s trailing
//!   blank line after them is conditional on whether *any* blocks follow
//!   (`has_directives && !doc.blocks.is_empty()`) — unknowable at the moment
//!   the `Metadata` event arrives. The five formatted lines (bounded, not
//!   O(document)) are held until the first subsequent event decides whether
//!   a blank line follows them.
//!
//! # Example
//! ```no_run
//! use muse_fmt::writer::Writer;
//! use muse_fmt::OwnedMuseEvent;
//! use std::borrow::Cow;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedMuseEvent::StartHeading { level: 1 });
//! w.write_event(OwnedMuseEvent::Text(Cow::Owned("Hello".to_string())));
//! w.write_event(OwnedMuseEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::OwnedMuseEvent;
use std::io::Write;

/// Streaming Muse writer.
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
    /// Buffered, pre-formatted title-page directive lines, awaiting the
    /// decision (made by the next event) of whether a trailing blank line
    /// follows. See the module doc's O(metadata field count) note.
    metadata_text: Option<String>,
}

enum Frame {
    Paragraph,
    Heading,
    Blockquote,
    Verse,
    CenteredBlock,
    RightBlock,
    List { ordered: bool, num: u32 },
    ListItem,
    DefinitionList,
    DefinitionTerm,
    DefinitionDesc,
    Table,
    TableRow { header: bool, cell_idx: u32 },
    TableCell,
    FootnoteDef,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Superscript,
    Subscript,
    Link,
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
            metadata_text: None,
        }
    }

    /// Feed one event to the writer. Writes bytes to the sink immediately
    /// whenever this event completes a top-level construct.
    pub fn write_event(&mut self, event: OwnedMuseEvent) {
        match &event {
            OwnedMuseEvent::Metadata {
                title,
                author,
                date,
                description,
                keywords,
            } => {
                let mut text = String::new();
                for (prefix, value) in [
                    ("#title ", title),
                    ("#author ", author),
                    ("#date ", date),
                    ("#desc ", description),
                    ("#keywords ", keywords),
                ] {
                    if let Some(v) = value {
                        text.push_str(prefix);
                        text.push_str(v);
                        text.push('\n');
                    }
                }
                self.metadata_text = Some(text);
                return;
            }
            OwnedMuseEvent::StartDocument => {}
            // `StartDocument` always precedes `Metadata` (see events.rs), so
            // it must not trigger the flush — that would take (and discard)
            // the not-yet-populated metadata slot.
            OwnedMuseEvent::EndDocument => self.flush_metadata_if_pending(false),
            _ => self.flush_metadata_if_pending(true),
        }
        self.process(event);
        self.maybe_flush();
    }

    /// Flush any remaining buffered bytes and recover the sink.
    pub fn finish(mut self) -> W {
        self.flush_metadata_if_pending(false);
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

    fn flush_metadata_if_pending(&mut self, more_content_follows: bool) {
        if let Some(text) = self.metadata_text.take()
            && !text.is_empty()
        {
            self.push(&text);
            if more_content_follows {
                self.push("\n");
            }
        }
    }

    /// Whether the current innermost frame accepts inline text/markup.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph
                    | Frame::Heading
                    | Frame::Bold
                    | Frame::Italic
                    | Frame::Underline
                    | Frame::Strikethrough
                    | Frame::Superscript
                    | Frame::Subscript
                    | Frame::Link
                    | Frame::TableCell
                    | Frame::DefinitionTerm
                    | Frame::FootnoteDef
            )
        )
    }

    /// Mirrors `emit::build_block`'s per-container handling of a `Paragraph`
    /// child: called with the stack already popped back to the parent
    /// frame, so `self.stack.last()` is that parent.
    fn paragraph_terminator(&self) -> &'static str {
        match self.stack.last() {
            Some(Frame::ListItem | Frame::DefinitionDesc) => "",
            Some(Frame::Blockquote | Frame::Verse | Frame::CenteredBlock | Frame::RightBlock) => {
                "\n"
            }
            _ => "\n\n",
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: OwnedMuseEvent) {
        match event {
            OwnedMuseEvent::StartDocument | OwnedMuseEvent::EndDocument => {}
            OwnedMuseEvent::Metadata { .. } => unreachable!("handled in write_event"),

            OwnedMuseEvent::StartParagraph => self.stack.push(Frame::Paragraph),
            OwnedMuseEvent::EndParagraph => {
                if matches!(self.stack.last(), Some(Frame::Paragraph)) {
                    self.stack.pop();
                    let term = self.paragraph_terminator();
                    self.push(term);
                }
            }
            OwnedMuseEvent::StartHeading { level } => {
                let capped = (level as usize).min(5);
                for _ in 0..capped {
                    self.push("*");
                }
                self.push(" ");
                self.stack.push(Frame::Heading);
            }
            OwnedMuseEvent::EndHeading => {
                if matches!(self.stack.last(), Some(Frame::Heading)) {
                    self.stack.pop();
                    self.push("\n\n");
                }
            }
            OwnedMuseEvent::StartBlockquote => {
                self.push("<quote>\n");
                self.stack.push(Frame::Blockquote);
            }
            OwnedMuseEvent::EndBlockquote => {
                if matches!(self.stack.last(), Some(Frame::Blockquote)) {
                    self.stack.pop();
                    self.push("</quote>\n\n");
                }
            }
            OwnedMuseEvent::StartVerse => {
                self.push("<verse>\n");
                self.stack.push(Frame::Verse);
            }
            OwnedMuseEvent::EndVerse => {
                if matches!(self.stack.last(), Some(Frame::Verse)) {
                    self.stack.pop();
                    self.push("</verse>\n\n");
                }
            }
            OwnedMuseEvent::StartCenteredBlock => {
                self.push("<center>\n");
                self.stack.push(Frame::CenteredBlock);
            }
            OwnedMuseEvent::EndCenteredBlock => {
                if matches!(self.stack.last(), Some(Frame::CenteredBlock)) {
                    self.stack.pop();
                    self.push("</center>\n\n");
                }
            }
            OwnedMuseEvent::StartRightBlock => {
                self.push("<right>\n");
                self.stack.push(Frame::RightBlock);
            }
            OwnedMuseEvent::EndRightBlock => {
                if matches!(self.stack.last(), Some(Frame::RightBlock)) {
                    self.stack.pop();
                    self.push("</right>\n\n");
                }
            }
            OwnedMuseEvent::StartList { ordered } => {
                self.stack.push(Frame::List { ordered, num: 1 });
            }
            OwnedMuseEvent::EndList => {
                if matches!(self.stack.last(), Some(Frame::List { .. })) {
                    self.stack.pop();
                    self.push("\n");
                }
            }
            OwnedMuseEvent::StartListItem => {
                match self.stack.last_mut() {
                    Some(Frame::List {
                        ordered: true, num, ..
                    }) => {
                        let prefix = format!(" {num}. ");
                        *num += 1;
                        self.push(&prefix);
                    }
                    Some(Frame::List { ordered: false, .. }) => self.push(" - "),
                    _ => {}
                }
                self.stack.push(Frame::ListItem);
            }
            OwnedMuseEvent::EndListItem => {
                if matches!(self.stack.last(), Some(Frame::ListItem)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }
            OwnedMuseEvent::StartDefinitionList => self.stack.push(Frame::DefinitionList),
            OwnedMuseEvent::EndDefinitionList => {
                if matches!(self.stack.last(), Some(Frame::DefinitionList)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }
            OwnedMuseEvent::StartDefinitionTerm => self.stack.push(Frame::DefinitionTerm),
            OwnedMuseEvent::EndDefinitionTerm => {
                if matches!(self.stack.last(), Some(Frame::DefinitionTerm)) {
                    self.stack.pop();
                    self.push(" :: ");
                }
            }
            OwnedMuseEvent::StartDefinitionDesc => self.stack.push(Frame::DefinitionDesc),
            OwnedMuseEvent::EndDefinitionDesc => {
                if matches!(self.stack.last(), Some(Frame::DefinitionDesc)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }
            OwnedMuseEvent::StartTable => self.stack.push(Frame::Table),
            OwnedMuseEvent::EndTable => {
                if matches!(self.stack.last(), Some(Frame::Table)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }
            OwnedMuseEvent::StartTableRow { header } => {
                self.push(if header { "|| " } else { "| " });
                self.stack.push(Frame::TableRow {
                    header,
                    cell_idx: 0,
                });
            }
            OwnedMuseEvent::EndTableRow => {
                if let Some(Frame::TableRow { header, .. }) = self.stack.pop() {
                    self.push(if header { " ||\n" } else { " |\n" });
                }
            }
            OwnedMuseEvent::StartTableCell => {
                if let Some(Frame::TableRow { header, cell_idx }) = self.stack.last_mut() {
                    let sep = if *cell_idx > 0 {
                        Some(if *header { " || " } else { " | " })
                    } else {
                        None
                    };
                    *cell_idx += 1;
                    if let Some(sep) = sep {
                        self.push(sep);
                    }
                }
                self.stack.push(Frame::TableCell);
            }
            OwnedMuseEvent::EndTableCell => {
                if matches!(self.stack.last(), Some(Frame::TableCell)) {
                    self.stack.pop();
                }
            }
            OwnedMuseEvent::StartFootnoteDef { label } => {
                self.push("[");
                self.push(&label);
                self.push("] ");
                self.stack.push(Frame::FootnoteDef);
            }
            OwnedMuseEvent::EndFootnoteDef => {
                if matches!(self.stack.last(), Some(Frame::FootnoteDef)) {
                    self.stack.pop();
                    self.push("\n\n");
                }
            }
            OwnedMuseEvent::HorizontalRule => self.push("----\n\n"),

            OwnedMuseEvent::LiteralBlock { content } => {
                self.push("<literal>\n");
                self.push(&content);
                if !content.ends_with('\n') {
                    self.push("\n");
                }
                self.push("</literal>\n\n");
            }
            OwnedMuseEvent::SrcBlock { lang, content } => {
                if let Some(lang) = &lang {
                    self.push(&format!("<src lang=\"{lang}\">\n"));
                } else {
                    self.push("<src>\n");
                }
                self.push(&content);
                if !content.ends_with('\n') {
                    self.push("\n");
                }
                self.push("</src>\n\n");
            }
            OwnedMuseEvent::CodeBlock { content } => {
                self.push("<example>\n");
                self.push(&content);
                if !content.ends_with('\n') {
                    self.push("\n");
                }
                self.push("</example>\n\n");
            }
            OwnedMuseEvent::Comment { content } => {
                self.push(";; ");
                self.push(&content);
                self.push("\n\n");
            }

            // ── Inline events ─────────────────────────────────────────────
            OwnedMuseEvent::Text(cow) => {
                if self.accepts_inline() {
                    self.push(&cow);
                }
            }
            OwnedMuseEvent::StartBold => self.write_delim_open("**", Frame::Bold),
            OwnedMuseEvent::EndBold => self.write_delim_close(&Frame::Bold, "**"),
            OwnedMuseEvent::StartItalic => self.write_delim_open("*", Frame::Italic),
            OwnedMuseEvent::EndItalic => self.write_delim_close(&Frame::Italic, "*"),
            OwnedMuseEvent::StartUnderline => self.write_delim_open("_", Frame::Underline),
            OwnedMuseEvent::EndUnderline => self.write_delim_close(&Frame::Underline, "_"),
            OwnedMuseEvent::StartStrikethrough => {
                self.write_delim_open("~~", Frame::Strikethrough);
            }
            OwnedMuseEvent::EndStrikethrough => {
                self.write_delim_close(&Frame::Strikethrough, "~~");
            }
            OwnedMuseEvent::StartSuperscript => self.write_delim_open("^", Frame::Superscript),
            OwnedMuseEvent::EndSuperscript => self.write_delim_close(&Frame::Superscript, "^"),
            OwnedMuseEvent::StartSubscript => self.write_delim_open("<sub>", Frame::Subscript),
            OwnedMuseEvent::EndSubscript => self.write_delim_close(&Frame::Subscript, "</sub>"),
            OwnedMuseEvent::Code(cow) => {
                if self.accepts_inline() {
                    self.push("=");
                    self.push(&cow);
                    self.push("=");
                }
            }
            OwnedMuseEvent::StartLink { url } => {
                if self.accepts_inline() {
                    self.push("[[");
                    self.push(&url);
                    self.push("][");
                }
                self.stack.push(Frame::Link);
            }
            OwnedMuseEvent::EndLink => {
                if matches!(self.stack.last(), Some(Frame::Link)) {
                    self.stack.pop();
                    if self.accepts_inline() {
                        self.push("]]");
                    }
                }
            }
            OwnedMuseEvent::FootnoteRef { label } => {
                if self.accepts_inline() {
                    self.push("[");
                    self.push(&label);
                    self.push("]");
                }
            }
            OwnedMuseEvent::LineBreak => {
                if self.accepts_inline() {
                    self.push("<br>");
                }
            }
            OwnedMuseEvent::Anchor { name } => {
                if self.accepts_inline() {
                    self.push("<anchor ");
                    self.push(&name);
                    self.push(">");
                }
            }
            OwnedMuseEvent::Image { src, alt } => {
                if self.accepts_inline() {
                    self.push("[[");
                    self.push(&src);
                    if let Some(alt) = &alt {
                        self.push("][");
                        self.push(alt);
                    }
                    self.push("]]");
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

    fn write_delim_close(&mut self, expected: &Frame, delim: &str) {
        let matches_top = matches!(
            (self.stack.last(), expected),
            (Some(Frame::Bold), Frame::Bold)
                | (Some(Frame::Italic), Frame::Italic)
                | (Some(Frame::Underline), Frame::Underline)
                | (Some(Frame::Strikethrough), Frame::Strikethrough)
                | (Some(Frame::Superscript), Frame::Superscript)
                | (Some(Frame::Subscript), Frame::Subscript)
        );
        if matches_top {
            self.stack.pop();
            // The closing delimiter belongs to the span that just closed,
            // mirroring `write_delim_open`'s unconditional emission.
            self.push(delim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMuseEvent::StartHeading { level: 1 });
        w.write_event(OwnedMuseEvent::Text(Cow::Owned("Hello".to_string())));
        w.write_event(OwnedMuseEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("* Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMuseEvent::StartParagraph);
        w.write_event(OwnedMuseEvent::Text(Cow::Owned("World".to_string())));
        w.write_event(OwnedMuseEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_bold() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMuseEvent::StartParagraph);
        w.write_event(OwnedMuseEvent::StartBold);
        w.write_event(OwnedMuseEvent::Text(Cow::Owned("strong".to_string())));
        w.write_event(OwnedMuseEvent::EndBold);
        w.write_event(OwnedMuseEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("**strong**"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "* Hello\n\nA paragraph with **bold** text.\n\n - item one\n - item two\n";
        let (doc, _) = crate::parse(input);
        let evts: Vec<_> = crate::events::events(&doc).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e.into_owned());
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        let (doc_orig, _) = crate::parse(input);
        let (doc_emit, _) = crate::parse(&emitted_text);
        assert_eq!(
            doc_orig.blocks.len(),
            doc_emit.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }

    #[test]
    fn test_writer_code_block() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMuseEvent::CodeBlock {
            content: Cow::Owned("fn main() {}".to_string()),
        });
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("<example>"), "got: {s:?}");
        assert!(s.contains("fn main() {}"), "got: {s:?}");
        assert!(s.contains("</example>"), "got: {s:?}");
    }

    #[test]
    fn test_writer_table() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMuseEvent::StartTable);
        w.write_event(OwnedMuseEvent::StartTableRow { header: true });
        w.write_event(OwnedMuseEvent::StartTableCell);
        w.write_event(OwnedMuseEvent::Text(Cow::Owned("Name".to_string())));
        w.write_event(OwnedMuseEvent::EndTableCell);
        w.write_event(OwnedMuseEvent::EndTableRow);
        w.write_event(OwnedMuseEvent::EndTable);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("||"), "got: {s:?}");
        assert!(s.contains("Name"), "got: {s:?}");
    }

    #[test]
    fn test_writer_metadata_title_page() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedMuseEvent::StartDocument);
        w.write_event(OwnedMuseEvent::Metadata {
            title: Some(Cow::Owned("My Doc".to_string())),
            author: Some(Cow::Owned("Jane".to_string())),
            date: None,
            description: None,
            keywords: None,
        });
        w.write_event(OwnedMuseEvent::StartParagraph);
        w.write_event(OwnedMuseEvent::Text(Cow::Owned("Body.".to_string())));
        w.write_event(OwnedMuseEvent::EndParagraph);
        w.write_event(OwnedMuseEvent::EndDocument);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert_eq!(s, "#title My Doc\n#author Jane\n\nBody.\n\n");
    }

    /// The streaming `Writer` must produce byte-identical output to the
    /// tree-based `emit::build` for the same document — including the
    /// context-dependent `Paragraph` terminator (top level vs list item vs
    /// blockquote/verse/centered/right), tables, definition lists, and a
    /// title page (via a hand-built event stream, since `parse()` doesn't
    /// yet feed `MuseDoc`'s title/author/etc. fields back through
    /// `events()` for arbitrary round-tripping beyond what's tested here).
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "* Hello\n\nA paragraph.\n",
            "A paragraph with **bold**, *italic*, _underline_, ~~strike~~, ^super^, <sub>sub</sub>, =code=, [[http://x/][a link]], [1], <br>, <anchor here>, [[img.png][alt text]].\n",
            " - item one\n - item two\n\n 1. first\n 2. second\n",
            "term1 :: definition one\nterm2 :: definition two\n",
            "<quote>\nA quoted paragraph.\n\nSecond paragraph.\n</quote>\n",
            "<verse>\nline one\nline two\n</verse>\n",
            "<center>\ncentered text\n</center>\n",
            "<right>\nright aligned\n</right>\n",
            "<example>\nfn main() {}\n</example>\n",
            "<src lang=\"rust\">\nfn main() {}\n</src>\n",
            "<literal>\nraw stuff\n</literal>\n",
            ";; a comment\n",
            "----\n",
            "|| Name || Age ||\n| Alice | 30 |\n",
            "[1] A footnote body.\n",
            " - outer\n\n   - inner a\n   - inner b\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse(input);
            let built = crate::emit::build(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::events::events(&doc) {
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

    /// Regression guard against reintroducing per-event `Block`/`Inline`
    /// subtree reconstruction, and (sharing the same allocator) a
    /// peak-memory-stays-roughly-constant guard, both as ratios between a
    /// small and large run rather than absolute thresholds — this test
    /// binary's threads share one process-wide `#[global_allocator]`, so an
    /// absolute threshold isn't robust to unrelated concurrent tests'
    /// allocation noise, but a genuine unbounded-growth bug still shows up
    /// as a large ratio between the two runs.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::cell::Cell;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct TrackingAlloc;
        static ALLOCS: AtomicUsize = AtomicUsize::new(0);
        // current/peak bytes are tracked per-thread (`thread_local!`, not a
        // shared `AtomicUsize`): the allocator is process-wide, and `cargo
        // test` runs other tests concurrently on other threads by default,
        // so a shared counter lets an unrelated test's allocations inflate
        // this measurement — confirmed as a real flake in this batch's
        // `pod-fmt` sibling (a spurious 407x ratio under full-workspace
        // `cargo test -q`, passing cleanly under `--test-threads=1`).
        // Thread-local counters make the measurement immune to what other
        // threads in the same binary do.
        thread_local! {
            static CURRENT_BYTES: Cell<usize> = const { Cell::new(0) };
            static PEAK_BYTES: Cell<usize> = const { Cell::new(0) };
        }
        unsafe impl GlobalAlloc for TrackingAlloc {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
                ALLOCS.fetch_add(1, Ordering::Relaxed);
                let cur = CURRENT_BYTES.with(|c| {
                    let v = c.get() + layout.size();
                    c.set(v);
                    v
                });
                PEAK_BYTES.with(|p| {
                    if cur > p.get() {
                        p.set(cur);
                    }
                });
                unsafe { System.alloc(layout) }
            }
            unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
                CURRENT_BYTES.with(|c| c.set(c.get().saturating_sub(layout.size())));
                unsafe { System.dealloc(ptr, layout) }
            }
        }
        #[global_allocator]
        static GLOBAL: TrackingAlloc = TrackingAlloc;

        fn events_for(n: usize) -> Vec<OwnedMuseEvent> {
            let mut evs = Vec::new();
            for i in 0..n {
                evs.push(OwnedMuseEvent::StartHeading { level: 2 });
                evs.push(OwnedMuseEvent::Text(Cow::Owned(format!("Section {i}"))));
                evs.push(OwnedMuseEvent::EndHeading);
                evs.push(OwnedMuseEvent::StartParagraph);
                evs.push(OwnedMuseEvent::Text(Cow::Owned("plain ".to_string())));
                evs.push(OwnedMuseEvent::StartBold);
                evs.push(OwnedMuseEvent::Text(Cow::Owned("bold".to_string())));
                evs.push(OwnedMuseEvent::EndBold);
                evs.push(OwnedMuseEvent::EndParagraph);
                evs.push(OwnedMuseEvent::StartList { ordered: false });
                for j in 0..2 {
                    evs.push(OwnedMuseEvent::StartListItem);
                    evs.push(OwnedMuseEvent::StartParagraph);
                    evs.push(OwnedMuseEvent::Text(Cow::Owned(format!("item {j}"))));
                    evs.push(OwnedMuseEvent::EndParagraph);
                    evs.push(OwnedMuseEvent::EndListItem);
                }
                evs.push(OwnedMuseEvent::EndList);
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
            let before = CURRENT_BYTES.with(|c| c.get());
            PEAK_BYTES.with(|p| p.set(before));
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
            PEAK_BYTES.with(|p| p.get()).saturating_sub(before)
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
