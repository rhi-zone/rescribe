//! Chunk-driven (batch) Muse parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all remaining events to the
//! handler.
//!
//! # Memory model
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input).
//! Use it when you need the complete AST.
//!
//! [`StreamingParser`] is genuinely incremental: it accumulates lines only
//! until a top-level block boundary is confirmed (a blank line, a line that
//! starts a different kind of block, a tag block's own closing tag, or a
//! single-line construct like a heading), then immediately re-parses just
//! that block's text via [`crate::parse::parse_blocks`] and forwards its
//! events to the handler — before `finish()` is ever called. Memory is
//! O(largest block), not O(full input): only the current in-progress block's
//! lines (plus at most one partial trailing line) are held at any time.
//!
//! Block-boundary classification (see [`crate::parse`]'s "Boundary
//! predicates" section) mirrors [`crate::parse::Parser::parse_block_loop`]'s
//! own dispatch order via shared pure predicate functions, so the two never
//! drift apart. Muse's tag blocks (`<example>`, `<verse>`, `<quote>`,
//! `<center>`, `<right>`, `<literal>`, `<src ...>`, `<comment>`) do not
//! support nesting in `parse()` itself (each looks for the *first*
//! occurrence of its own closing tag, regardless of any nested same-tag
//! open), so `StreamingParser` intentionally does not track nesting depth
//! either — that would produce output that diverges from `events()`.
//!
//! # Example — AST style
//! ```no_run
//! use muse_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"* Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use muse_fmt::batch::{StreamingParser, Handler};
//! use muse_fmt::OwnedMuseEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedMuseEvent| events.push(ev));
//! p.feed(b"* Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use std::collections::VecDeque;

use crate::ast::{Diagnostic, MuseDoc};
use crate::events::OwnedMuseEvent;
use crate::parse::{
    heading_level, is_definition_list_line, is_footnote_def_start, is_horizontal_rule,
    is_indented_code_start, is_ordered_list_item, is_over_leveled_heading, is_table_row,
    is_unordered_list_start, tag_open_close,
};

/// Chunk-driven Muse parser that returns the full AST on finish.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    pub fn new() -> Self {
        BatchParser { buf: Vec::new() }
    }

    /// Feed a chunk of input bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish parsing and return the AST.
    pub fn finish(self) -> (MuseDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming Muse events.
///
/// Implemented automatically for any `FnMut(OwnedMuseEvent)`.
pub trait Handler {
    fn handle(&mut self, event: OwnedMuseEvent);
}

impl<F: FnMut(OwnedMuseEvent)> Handler for F {
    fn handle(&mut self, event: OwnedMuseEvent) {
        self(event);
    }
}

// ── Block-boundary classification ───────────────────────────────────────────

/// What kind of top-level block a line, seen with no pending block, starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classify {
    Blank,
    /// A construct that is always exactly one line: comment line, footnote
    /// definition, heading, or horizontal rule.
    SingleLine,
    /// A line consumed with no block produced at all (unknown block tag,
    /// over-leveled heading).
    SkipLine,
    /// Opens a tag-delimited block; carries the block's closing tag.
    TagOpen(&'static str),
    /// Starts (or continues) a multi-line, blank/mismatch-terminated block.
    Continued(ContKind),
}

/// A block kind whose lines accumulate until a line fails its continuation
/// predicate (which always includes "line is blank", except
/// [`ContKind::IndentedCode`] — see [`ContKind::continues`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContKind {
    Paragraph,
    UnorderedList,
    OrderedList,
    DefinitionList,
    IndentedCode,
    Table,
}

impl ContKind {
    /// True if `line` continues a block of this kind. Mirrors the `while`
    /// condition of the corresponding `Parser::parse_*` method exactly.
    fn continues(self, line: &str) -> bool {
        match self {
            ContKind::Table => is_table_row(line),
            ContKind::UnorderedList => is_unordered_list_start(line),
            ContKind::OrderedList => is_ordered_list_item(line),
            ContKind::DefinitionList => is_definition_list_line(line),
            // parse_indented_code's own while-loop does NOT stop at blank
            // lines (unlike every other Continued kind) — it only breaks on
            // a non-blank line that doesn't start with two spaces.
            ContKind::IndentedCode => is_indented_code_start(line) || line.trim().is_empty(),
            // parse_paragraph's break-condition list is, line for line,
            // exactly "this line classifies as something other than a
            // paragraph start" — see the comment on `classify_line`.
            ContKind::Paragraph => classify_line(line) == Classify::Continued(ContKind::Paragraph),
        }
    }
}

/// Classify a line with no pending block, in the same order as
/// [`crate::parse::Parser::parse_block_loop`]'s own `if`/`else if` chain.
fn classify_line(line: &str) -> Classify {
    if line.trim().is_empty() {
        return Classify::Blank;
    }
    if line.starts_with(";; ") || line == ";;" {
        return Classify::SingleLine;
    }
    if is_footnote_def_start(line) {
        return Classify::SingleLine;
    }
    if is_table_row(line) {
        return Classify::Continued(ContKind::Table);
    }
    if let Some(close) = tag_open_close(line) {
        return Classify::TagOpen(close);
    }
    if line.trim_start().starts_with('<') && !crate::parse::is_inline_tag_line(line) {
        return Classify::SkipLine;
    }
    if heading_level(line).is_some() {
        return Classify::SingleLine;
    }
    if is_over_leveled_heading(line) {
        return Classify::SkipLine;
    }
    if is_horizontal_rule(line) {
        return Classify::SingleLine;
    }
    if is_unordered_list_start(line) {
        return Classify::Continued(ContKind::UnorderedList);
    }
    if is_ordered_list_item(line) {
        return Classify::Continued(ContKind::OrderedList);
    }
    if is_definition_list_line(line) {
        return Classify::Continued(ContKind::DefinitionList);
    }
    if is_indented_code_start(line) {
        return Classify::Continued(ContKind::IndentedCode);
    }
    Classify::Continued(ContKind::Paragraph)
}

/// Pending block-accumulation state.
enum Pending {
    None,
    /// Inside a tag block (`<example>`, `<verse>`, ...), looking for `close`.
    Tag {
        close: &'static str,
        lines: Vec<String>,
    },
    /// Accumulating a `Continued`-kind block.
    Cont {
        kind: ContKind,
        lines: Vec<String>,
    },
}

/// Chunked streaming Muse parser that delivers events to a [`Handler`]
/// incrementally, as each top-level block boundary is confirmed.
///
/// Memory: O(largest block). See the module docs for the block-boundary
/// design and its relationship to [`crate::parse::Parser::parse_block_loop`].
pub struct StreamingParser<H: Handler> {
    handler: H,
    /// Raw bytes not yet forming a complete line. `\n` is never a
    /// continuation byte in valid UTF-8, so splitting on raw `\n` bytes
    /// before decoding is always safe, even mid-multi-byte-character.
    line_buf: Vec<u8>,
    pending: Pending,
    header_done: bool,
    title: Option<String>,
    author: Option<String>,
    date: Option<String>,
    description: Option<String>,
    keywords: Option<String>,
}

impl<H: Handler> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    /// Emits `StartDocument` immediately.
    pub fn new(mut handler: H) -> Self {
        handler.handle(OwnedMuseEvent::StartDocument);
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            pending: Pending::None,
            header_done: false,
            title: None,
            author: None,
            date: None,
            description: None,
            keywords: None,
        }
    }

    /// Feed a chunk of bytes. May call `handler.handle()` zero or more times.
    pub fn feed(&mut self, chunk: &[u8]) {
        for &byte in chunk {
            if byte == b'\n' {
                if self.line_buf.last() == Some(&b'\r') {
                    self.line_buf.pop();
                }
                let line = String::from_utf8_lossy(&self.line_buf).into_owned();
                self.line_buf.clear();
                self.feed_line(line);
            } else {
                self.line_buf.push(byte);
            }
        }
    }

    fn feed_line(&mut self, line: String) {
        if !self.header_done {
            if line.trim().is_empty() {
                // Header phase skips (but does not end on) blank lines.
                return;
            }
            if let Some(v) = line.strip_prefix("#title ") {
                self.title = Some(v.trim().to_string());
                return;
            }
            if let Some(v) = line.strip_prefix("#author ") {
                self.author = Some(v.trim().to_string());
                return;
            }
            if let Some(v) = line.strip_prefix("#date ") {
                self.date = Some(v.trim().to_string());
                return;
            }
            if let Some(v) = line.strip_prefix("#desc ") {
                self.description = Some(v.trim().to_string());
                return;
            }
            if let Some(v) = line.strip_prefix("#keywords ") {
                self.keywords = Some(v.trim().to_string());
                return;
            }
            self.flush_header();
            // Fall through: this line is not a header directive, process it
            // as the start of the normal block stream.
        }
        self.feed_line_normal(line);
    }

    fn flush_header(&mut self) {
        if self.header_done {
            return;
        }
        self.header_done = true;
        self.handler.handle(OwnedMuseEvent::Metadata {
            title: self.title.take().map(std::borrow::Cow::Owned),
            author: self.author.take().map(std::borrow::Cow::Owned),
            date: self.date.take().map(std::borrow::Cow::Owned),
            description: self.description.take().map(std::borrow::Cow::Owned),
            keywords: self.keywords.take().map(std::borrow::Cow::Owned),
        });
    }

    fn feed_line_normal(&mut self, line: String) {
        match &mut self.pending {
            Pending::Tag { close, lines } => {
                let close = *close;
                if let Some(idx) = line.find(close) {
                    lines.push(line[..idx].to_string());
                    self.flush_pending();
                } else {
                    lines.push(line);
                }
                return;
            }
            Pending::Cont { kind, lines } => {
                if kind.continues(&line) {
                    lines.push(line);
                    return;
                }
                self.flush_pending();
                // Fall through: reclassify this line as a fresh block start.
            }
            Pending::None => {}
        }

        match classify_line(&line) {
            Classify::Blank => {}
            Classify::SkipLine => {}
            Classify::SingleLine => self.emit_text(&line),
            Classify::TagOpen(close) => {
                if line.contains(close) {
                    // Opens and closes on the same physical line.
                    self.emit_text(&line);
                } else {
                    self.pending = Pending::Tag {
                        close,
                        lines: vec![line],
                    };
                }
            }
            Classify::Continued(kind) => {
                self.pending = Pending::Cont {
                    kind,
                    lines: vec![line],
                };
            }
        }
    }

    /// Flush whatever is pending (a `Continued` block or a `Tag` block),
    /// re-parse its accumulated text in isolation, and forward its events.
    fn flush_pending(&mut self) {
        let lines = match std::mem::replace(&mut self.pending, Pending::None) {
            Pending::None => return,
            Pending::Tag { lines, .. } => lines,
            Pending::Cont { lines, .. } => lines,
        };
        if lines.is_empty() {
            return;
        }
        let text = lines.join("\n");
        self.emit_text(&text);
    }

    /// Re-parse `text` — a single already-boundary-delimited block (or a
    /// single line) — via [`crate::parse::parse_blocks`] (which skips the
    /// document-header phase, since header directives are only meaningful
    /// at the very start of the whole document, already handled separately
    /// above) and forward its events to the handler.
    fn emit_text(&mut self, text: &str) {
        let (blocks, _diags) = crate::parse::parse_blocks(text);
        for block in &blocks {
            let mut q = VecDeque::new();
            crate::events::enqueue_block(block, &mut q);
            for event in q {
                self.handler.handle(event.into_owned());
            }
        }
    }

    /// Flush any remaining input and deliver final events, including
    /// `EndDocument`.
    pub fn finish(mut self) {
        if !self.line_buf.is_empty() {
            if self.line_buf.last() == Some(&b'\r') {
                self.line_buf.pop();
            }
            let line = String::from_utf8_lossy(&self.line_buf).into_owned();
            self.feed_line(line);
        }
        if !self.header_done {
            self.flush_header();
        }
        self.flush_pending();
        self.handler.handle(OwnedMuseEvent::EndDocument);
    }
}

/// Chunk-driven Muse parser that delivers events to a callback on finish.
///
/// Convenience wrapper around [`StreamingParser`] for closure-based usage.
/// Unlike [`StreamingParser`], this buffers all input and is O(full input) —
/// prefer `StreamingParser` for large inputs.
pub struct BatchSink<F: FnMut(OwnedMuseEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(OwnedMuseEvent)> BatchSink<F> {
    pub fn new(callback: F) -> Self {
        BatchSink {
            buf: Vec::new(),
            callback,
        }
    }

    /// Feed a chunk of input bytes.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Finish parsing and deliver all events to the callback.
    pub fn finish(mut self) {
        let s = String::from_utf8_lossy(&self.buf);
        let (doc, _) = crate::parse::parse(&s);
        for event in crate::events::events(&doc) {
            (self.callback)(event.into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedMuseEvent;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b"* Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"* Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_matches_parse() {
        let input = "* Heading\n\nA paragraph.\n\n - item1\n - item2\n";
        let (doc_direct, _) = crate::parse(input);
        let mut p = BatchParser::new();
        p.feed(input.as_bytes());
        let (doc_batch, _) = p.finish();
        assert_eq!(doc_direct.blocks.len(), doc_batch.blocks.len());
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"* Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedMuseEvent::StartHeading { level: 1 }))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedMuseEvent::StartParagraph))
        );
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedMuseEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"* Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedMuseEvent::StartHeading { .. }))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedMuseEvent::StartParagraph))
        );
    }

    #[test]
    fn test_streaming_matches_events() {
        let input = b"* Heading\n\nParagraph one.\n\nParagraph two.\n";
        let s = String::from_utf8_lossy(input);
        let (doc, _) = crate::parse(&s);
        let bulk: Vec<OwnedMuseEvent> = crate::events::events(&doc)
            .map(|e| e.into_owned())
            .collect();

        let mut streamed: Vec<OwnedMuseEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        for chunk in input.chunks(7) {
            p.feed(chunk);
        }
        p.finish();

        assert_eq!(bulk, streamed);
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev| events.push(ev));
        sink.feed(b"* Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedMuseEvent::StartHeading { level: 1 }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedMuseEvent::StartParagraph))
        );
    }

    // ── Incrementality ──────────────────────────────────────────────────

    /// Feeding half of a multi-block document must deliver events to the
    /// handler *before* `finish()` is called — the direct regression guard
    /// against the old buffer-then-finish behavior.
    #[test]
    fn test_streaming_parser_delivers_before_finish() {
        let input = b"* Heading one\n\nParagraph one.\n\n* Heading two\n\nParagraph two.\n";
        let mut delivered: Vec<OwnedMuseEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| delivered.push(ev));
        let mid = input.len() / 2;
        p.feed(&input[..mid]);
        assert!(
            !delivered.is_empty(),
            "expected events delivered before finish() from half the input"
        );
        // Drop without calling finish(): this probe only needs pre-finish
        // handler state.
    }

    fn bulk_events(input: &[u8]) -> Vec<OwnedMuseEvent> {
        let s = String::from_utf8_lossy(input);
        let (doc, _) = crate::parse(&s);
        crate::events::events(&doc)
            .map(|e| e.into_owned())
            .collect()
    }

    fn streamed_events_chunked(input: &[u8], chunk_size: usize) -> Vec<OwnedMuseEvent> {
        let mut streamed = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        if chunk_size == 0 {
            p.feed(input);
        } else {
            for chunk in input.chunks(chunk_size) {
                p.feed(chunk);
            }
        }
        p.finish();
        streamed
    }

    /// Construct mix covering headings, paragraphs, lists, tables, tag
    /// blocks (including a same-line-close and a same-line-close with
    /// attributes), definition lists, indented code, footnotes, comments,
    /// horizontal rules, and a document header.
    fn adversarial_input() -> &'static [u8] {
        b"#title Streaming Test\n\
          #author Jane Doe\n\
          #date 2024-01-01\n\
          \n\
          * Heading one\n\
          \n\
          A paragraph with **bold** and *em* text.\n\
          \n\
          ** Sub heading\n\
          \n\
          <example>\n\
          code line one\n\
          code line two\n\
          </example>\n\
          \n\
          <verse>\n\
          Line one\n\
          \n\
          Line two (blank line inside verse)\n\
          </verse>\n\
          \n\
          <src lang=\"rust\">fn main() {}</src>\n\
          \n\
          ;; a line comment\n\
          \n\
          <comment>\n\
          block comment\n\
          </comment>\n\
          \n\
          [1] A footnote definition.\n\
          \n\
          See [1] for details.\n\
          \n\
          || Name || Age ||\n\
          | Alice | 30 |\n\
          | Bob | 25 |\n\
          \n\
          - not a list (no leading space)\n\
          \n\
          term :: description\n\
          \n\
          ----\n\
          \n\
          After the rule.\n\
          \n\
          Indented:\n\
          \n\
          "
    }

    #[test]
    fn test_streaming_matches_events_adversarial_whole() {
        let input = adversarial_input();
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 0));
    }

    #[test]
    fn test_streaming_matches_events_adversarial_single_byte() {
        let input = adversarial_input();
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 1));
    }

    #[test]
    fn test_streaming_matches_events_adversarial_chunks_of_7() {
        let input = adversarial_input();
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 7));
    }

    #[test]
    fn test_streaming_matches_events_adversarial_chunks_of_37() {
        let input = adversarial_input();
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 37));
    }

    /// "café" — the 'é' is 2 UTF-8 bytes; split it across separate `feed()`
    /// calls (single-byte chunking already tears every multi-byte char
    /// apart, but this is the minimal, explicit case).
    #[test]
    fn test_streaming_split_mid_utf8_char() {
        let input = "café society\n\nAnother caf\u{e9} paragraph.\n".as_bytes();
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 1));
    }

    #[test]
    fn test_streaming_split_mid_tag_block() {
        let input = b"<example>\nfn main() {\n    1 + 1\n}\n</example>\n\nAfter.\n";
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 3));
    }

    #[test]
    fn test_streaming_nested_same_tag_not_supported_matches_events() {
        // parse() itself does not support nesting same-type tag blocks
        // (parse_verse_block etc. stop at the *first* occurrence of their
        // own closing tag) — StreamingParser must reproduce that, not "fix"
        // it, to stay byte-for-byte aligned with events().
        let input = b"<verse>\nouter\n<verse>\ninner\n</verse>\nmore\n</verse>\n";
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 5));
    }

    #[test]
    fn test_streaming_empty_input() {
        assert_eq!(bulk_events(b""), streamed_events_chunked(b"", 0));
    }

    #[test]
    fn test_streaming_header_only() {
        let input = b"#title Only A Header\n#author Someone\n";
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 4));
    }

    #[test]
    fn test_streaming_non_header_hash_line_mid_document() {
        // A '#'-led line that is NOT at the document start must not be
        // reinterpreted as a header directive by the per-block re-parse.
        let input = b"* Heading\n\n#not a directive, just text\n";
        assert_eq!(bulk_events(input), streamed_events_chunked(input, 6));
    }

    // ── Memory guard ─────────────────────────────────────────────────────
    //
    // A process may only define one `#[global_allocator]` per binary — this
    // reuses the crate-wide test-only instrumented allocator in
    // `crate::alloc_probe` (also used by `writer.rs`'s peak-memory guard)
    // instead of declaring a second one here.

    fn synthetic_muse(paragraphs: usize) -> Vec<u8> {
        let mut s = String::new();
        for i in 0..paragraphs {
            s.push_str(&format!("* Section {i}\n\n"));
            s.push_str("A paragraph with **bold** and *em* text repeated a few times ");
            s.push_str("to give the block some real size before the next boundary. ");
            s.push_str("Another sentence here to pad it out further still.\n\n");
            s.push_str(" - item one\n - item two\n - item three\n\n");
        }
        s.into_bytes()
    }

    /// Peak-memory guard: feeding a large synthetic document through
    /// `StreamingParser` in small chunks must keep peak allocated bytes
    /// within a small constant multiple of a single top-level block's size,
    /// not grow with the full document. The old buffer-then-finish
    /// `StreamingParser` held the *entire* input (plus the fully
    /// materialized `MuseDoc`) in memory simultaneously — O(full document).
    ///
    /// Measured as a scaling comparison (peak at 10x the paragraph count vs
    /// peak at baseline), not an absolute byte threshold: the allocator is
    /// process-wide, so an absolute threshold is at the mercy of unrelated
    /// tests allocating concurrently on other threads. A true O(full
    /// document) implementation shows peak growing ~10x; a bounded one
    /// stays flat.
    #[test]
    fn test_streaming_parser_peak_memory_bounded() {
        use crate::alloc_probe::{CURRENT, PEAK};

        fn measure_peak(paragraphs: usize) -> usize {
            let input = synthetic_muse(paragraphs);
            CURRENT.with(|c| c.set(0));
            PEAK.with(|p| p.set(0));
            let mut p = StreamingParser::new(|ev: OwnedMuseEvent| {
                std::hint::black_box(&ev);
            });
            for chunk in input.chunks(64) {
                p.feed(chunk);
            }
            p.finish();
            PEAK.with(|p| p.get())
        }

        let small = measure_peak(50).max(1);
        let large = measure_peak(500); // 10x the sections

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 4.0,
            "peak memory did not stay bounded: {small} bytes @50 sections -> {large} bytes \
             @500 sections (ratio {ratio:.2}); this suggests StreamingParser is buffering the \
             whole document again"
        );
    }
}
