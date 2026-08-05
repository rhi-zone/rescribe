//! Chunk-driven (batch) Texinfo parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! [`StreamingParser`] processes input in logical top-level units — a
//! paragraph, a heading, or an `@directive ... @end directive`-delimited
//! environment (list, definition list, multitable, code block, quotation,
//! menu, float, conditional block). Memory usage is O(largest unit in the
//! document): the full document is never held in memory simultaneously.
//! Each unit is flushed (parsed via [`crate::events::events`] and forwarded
//! to the handler) as soon as its boundary is confirmed by the input seen so
//! far.
//!
//! [`crate::parse::parse`] itself has no notion of nested same-type
//! environments (e.g. `@itemize` inside `@itemize` is not correctly
//! recognized — the outer scan stops at the *first* `@end itemize`, whether
//! it belongs to the outer or inner list). This splitter deliberately
//! mirrors that same flat (non-nesting) boundary detection rather than
//! second-guessing it: doing otherwise would produce different top-level
//! unit boundaries than a full-document [`crate::parse::parse`] call would,
//! which would make [`StreamingParser`]'s output diverge from
//! [`crate::events::events`] on the same input — the opposite of what
//! "genuinely incremental but behaviorally identical" streaming requires.
//!
//! `@settitle` is document-level metadata: [`crate::events::events`] always
//! emits a single `Title` event before any block events, regardless of
//! where in the source the `@settitle` line appears. This splitter buffers
//! the most recently seen title and emits it once, immediately before the
//! first content-producing event — which is correct for the (universal, and
//! Texinfo's own authoring convention) case where `@settitle` appears before
//! any other content. A `@settitle` occurring *after* content has already
//! been flushed to the handler cannot be moved earlier without buffering the
//! whole document, so such a (pathological, convention-violating) document
//! is a known limitation of the streaming path; [`BatchParser`] (which
//! buffers the full document) is unaffected.
//!
//! [`BatchParser`] buffers all input until `finish()` and is O(full input) —
//! this is intentional: producing the full AST requires the whole document
//! regardless of how it is fed in.
//!
//! # Example — AST style
//! ```no_run
//! use texinfo::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"@chapter Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use texinfo::batch::{StreamingParser, Handler};
//! use texinfo::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"@chapter Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, TexinfoDoc};
use crate::events::OwnedEvent;
use crate::parse::{
    code_block_end_marker, is_inline_command_at_start, try_code_block_start, try_conditional_start,
    try_heading,
};

/// Chunk-driven Texinfo parser that returns the full AST on finish.
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
    pub fn finish(self) -> (TexinfoDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Handler trait for streaming Texinfo events — the shared
/// [`rescribe_format_api::Handler`], not a locally declared trait.
pub use rescribe_format_api::Handler;

/// Directive prefixes that [`crate::parse::parse`]'s top-level loop consumes
/// without producing a block. Kept verbatim in sync with the equivalent list
/// in `parse.rs`'s `Parser::run`.
const SKIP_DIRECTIVE_PREFIXES: &[&str] = &[
    "@set ",
    "@clear ",
    "@include ",
    "@setfilename ",
    "@copying",
    "@end copying",
    "@titlepage",
    "@end titlepage",
    "@contents",
    "@shortcontents",
    "@summarycontents",
    "@top",
    "@bye",
    "@dircategory",
    "@direntry",
    "@end direntry",
    "\\input ",
    "@author ",
];

/// The kind of `@directive ... @end directive`-delimited environment
/// currently being accumulated, and the condition (mirroring the matching
/// scanner function in `parse.rs`) that ends it.
enum EnvKind {
    /// `@itemize` / `@enumerate` — ends at a line (after full `trim()`)
    /// starting with `@end itemize` or `@end enumerate` (matches either,
    /// same as `parse_list`).
    List,
    /// `@table` — ends at a line starting with `@end table`.
    DefList,
    /// `@multitable` — ends at a line starting with `@end multitable`.
    Multitable,
    /// A code-block variant — ends at a line exactly equal (after
    /// `trim()`) to `end_marker`.
    CodeBlock { end_marker: String },
    /// `@quotation` — ends at a line starting with `@end quotation`.
    Quotation,
    /// `@menu` — ends at a line starting with `@end menu`.
    Menu,
    /// `@float` — ends at a line starting with `@end float`.
    Float,
    /// A conditional block (`@iftex`, `@ifnothtml`, ...) — ends at a line
    /// exactly equal (after `trim()`) to `end_marker`.
    Conditional { end_marker: String },
}

impl EnvKind {
    /// Whether `trimmed` (the current line after full `trim()`) ends this
    /// environment, matching the corresponding scanner in `parse.rs`.
    fn is_end(&self, trimmed: &str) -> bool {
        match self {
            EnvKind::List => {
                trimmed.starts_with("@end itemize") || trimmed.starts_with("@end enumerate")
            }
            EnvKind::DefList => trimmed.starts_with("@end table"),
            EnvKind::Multitable => trimmed.starts_with("@end multitable"),
            EnvKind::CodeBlock { end_marker } => trimmed == end_marker,
            EnvKind::Quotation => trimmed.starts_with("@end quotation"),
            EnvKind::Menu => trimmed.starts_with("@end menu"),
            EnvKind::Float => trimmed.starts_with("@end float"),
            EnvKind::Conditional { end_marker } => trimmed == end_marker,
        }
    }
}

/// Accumulation state for the streaming parser.
enum Mode {
    /// Waiting for the first line of the next top-level unit.
    Idle,
    /// Accumulating a paragraph. Ends at a blank line, or a line starting
    /// with `@` that is not an inline command (mirrors `collect_paragraph`).
    Paragraph,
    /// Accumulating an `@directive ... @end directive` environment.
    Env(EnvKind),
}

/// Chunked streaming Texinfo parser that delivers events to a [`Handler`]
/// incrementally, as soon as each top-level unit's boundary is confirmed.
///
/// Memory: O(largest unit in the document). See the [module-level
/// docs](self) for the exact boundary rules and known limitations.
pub struct StreamingParser<H: Handler<OwnedEvent>> {
    handler: H,
    /// Bytes of the current incomplete line (not yet terminated by `\n`).
    line_buf: Vec<u8>,
    /// Raw (untrimmed) lines accumulated for the unit currently in
    /// progress.
    pending_lines: Vec<String>,
    mode: Mode,
    /// Most recently seen `@settitle` text, not yet emitted.
    title: Option<String>,
    /// Whether the `Title` event has already been forwarded.
    title_emitted: bool,
}

impl<H: Handler<OwnedEvent>> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            pending_lines: Vec::new(),
            mode: Mode::Idle,
            title: None,
            title_emitted: false,
        }
    }

    /// Feed a chunk of bytes. May call `handler.handle()` zero or more
    /// times.
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

    /// Route `line` (raw, no trailing `\n`) according to the current mode.
    fn feed_line(&mut self, line: String) {
        match &self.mode {
            Mode::Env(kind) => {
                let trimmed = line.trim();
                let ends = kind.is_end(trimmed);
                self.pending_lines.push(line);
                if ends {
                    self.flush_pending();
                    self.mode = Mode::Idle;
                }
            }
            Mode::Paragraph => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    self.flush_pending();
                    self.mode = Mode::Idle;
                } else if trimmed.starts_with('@') && !is_inline_command_at_start(trimmed) {
                    self.flush_pending();
                    self.mode = Mode::Idle;
                    // Re-dispatch this line: it terminates the paragraph
                    // without being part of it (mirrors `collect_paragraph`
                    // breaking without consuming the line).
                    self.dispatch_line(line);
                } else {
                    self.pending_lines.push(line);
                }
            }
            Mode::Idle => self.dispatch_line(line),
        }
    }

    /// Decide what kind of top-level unit `line` begins, matching
    /// `Parser::run`'s dispatch order exactly (using `trim_start()`, same as
    /// `parse.rs`).
    fn dispatch_line(&mut self, line: String) {
        let t = line.trim_start();

        // Comments.
        if t.starts_with("@c ") || t.starts_with("@comment ") || t == "@c" {
            return;
        }

        // @settitle.
        if let Some(title) = t.strip_prefix("@settitle ") {
            self.title = Some(title.trim().to_string());
            return;
        }

        // Directives skipped outright.
        if SKIP_DIRECTIVE_PREFIXES
            .iter()
            .any(|prefix| t.starts_with(prefix))
        {
            return;
        }

        // @node.
        if t.starts_with("@node ") || t == "@node" {
            return;
        }

        // @noindent — single-line unit.
        if t == "@noindent" || t.starts_with("@noindent ") {
            self.pending_lines.push(line);
            self.flush_pending();
            return;
        }

        // Headings — single-line unit.
        if try_heading(t).is_some() {
            self.pending_lines.push(line);
            self.flush_pending();
            return;
        }

        // Lists.
        if t.starts_with("@itemize") || t.starts_with("@enumerate") {
            self.pending_lines.push(line);
            self.mode = Mode::Env(EnvKind::List);
            return;
        }

        // Definition lists.
        if t.starts_with("@table") {
            self.pending_lines.push(line);
            self.mode = Mode::Env(EnvKind::DefList);
            return;
        }

        // Multitables.
        if t.starts_with("@multitable") {
            self.pending_lines.push(line);
            self.mode = Mode::Env(EnvKind::Multitable);
            return;
        }

        // Code blocks.
        if let Some(variant) = try_code_block_start(t) {
            let end_marker = code_block_end_marker(&variant);
            self.pending_lines.push(line);
            self.mode = Mode::Env(EnvKind::CodeBlock { end_marker });
            return;
        }

        // Quotations.
        if t.starts_with("@quotation") {
            self.pending_lines.push(line);
            self.mode = Mode::Env(EnvKind::Quotation);
            return;
        }

        // Menus.
        if t.starts_with("@menu") {
            self.pending_lines.push(line);
            self.mode = Mode::Env(EnvKind::Menu);
            return;
        }

        // Floats.
        if t.starts_with("@float") {
            self.pending_lines.push(line);
            self.mode = Mode::Env(EnvKind::Float);
            return;
        }

        // Conditional blocks.
        if let Some(env_name) = try_conditional_start(t) {
            let end_marker = format!("@end {}", env_name);
            self.pending_lines.push(line);
            self.mode = Mode::Env(EnvKind::Conditional { end_marker });
            return;
        }

        // Blank lines.
        if t.is_empty() {
            return;
        }

        // Unknown top-level `@`-directive: `collect_paragraph` breaks
        // immediately without consuming it, and `Parser::run` then advances
        // past it without pushing a block — silently dropped, same as full-
        // document parsing.
        if t.starts_with('@') && !is_inline_command_at_start(t) {
            return;
        }

        // Otherwise: start a paragraph.
        self.pending_lines.push(line);
        self.mode = Mode::Paragraph;
    }

    /// Parse the accumulated unit and deliver its events to the handler,
    /// emitting a deferred `Title` event first if this is the first
    /// content-producing flush.
    fn flush_pending(&mut self) {
        if self.pending_lines.is_empty() {
            return;
        }
        let text = self.pending_lines.join("\n");
        self.pending_lines.clear();

        if let Some(title) = self.title.take_if(|_| !self.title_emitted) {
            self.handler.handle(OwnedEvent::Title(title));
            self.title_emitted = true;
        }

        for event in crate::events::events(&text) {
            self.handler.handle(event.into_owned());
        }
    }

    /// Flush any remaining input and deliver final events.
    pub fn finish(mut self) {
        if !self.line_buf.is_empty() {
            if self.line_buf.last() == Some(&b'\r') {
                self.line_buf.pop();
            }
            let line = String::from_utf8_lossy(&self.line_buf).into_owned();
            self.feed_line(line);
        }
        self.flush_pending();
        if let Some(title) = self.title.take_if(|_| !self.title_emitted) {
            self.handler.handle(OwnedEvent::Title(title));
        }
    }
}

/// Chunk-driven Texinfo parser that delivers events to a callback on finish.
///
/// Prefer [`StreamingParser`] for new code; `BatchSink` is kept for
/// backwards compatibility. Unlike [`StreamingParser`], `BatchSink` buffers
/// all input until `finish()` (O(full input)) — it is a deliberately simple
/// back-compat shim, not the incremental API.
pub struct BatchSink<F: FnMut(OwnedEvent)> {
    buf: Vec<u8>,
    callback: F,
}

impl<F: FnMut(OwnedEvent)> BatchSink<F> {
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
        for event in crate::events::events(&s) {
            (self.callback)(event.into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedEvent;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b"@chapter Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_batch_parser_split_chunks() {
        let mut p = BatchParser::new();
        for b in b"@chapter Title\n\nContent here.\n" {
            p.feed(std::slice::from_ref(b));
        }
        let (doc, _) = p.finish();
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"@chapter Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. }))
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_parser_split_chunks() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        for b in b"@chapter Title\n\nContent.\n" {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { .. }))
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_streaming_matches_bulk() {
        let input = b"@chapter Heading\n\nParagraph one.\n\nParagraph two.\n";

        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events::events(&s).map(|e| e.into_owned()).collect()
        };

        let mut streamed: Vec<OwnedEvent> = Vec::new();
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
        sink.feed(b"@chapter Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1, .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartParagraph))
        );
    }

    // ── Incrementality ───────────────────────────────────────────────────

    /// `feed()` must deliver events to the handler *before* `finish()` is
    /// called, proving the parser advances real state per-chunk rather than
    /// buffering everything until `finish()`.
    #[test]
    fn test_streaming_parser_delivers_before_finish() {
        let mut evs: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        p.feed(b"@chapter Hello\n\n");
        assert!(
            !evs.is_empty(),
            "expected events to be delivered after feed() with a complete unit, before finish()"
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { .. }))
        );
        // Drop without calling finish(): only checking pre-finish delivery.
    }

    #[test]
    fn test_streaming_parser_env_with_blank_lines_not_split() {
        // A @quotation containing a blank line must not be split into two
        // units at the blank line.
        let input = b"@quotation\nFirst para.\n\nSecond para.\n@end quotation\n";
        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events::events(&s).map(|e| e.into_owned()).collect()
        };
        let mut streamed: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        p.feed(input);
        p.finish();
        assert_eq!(bulk, streamed);
        assert_eq!(
            streamed
                .iter()
                .filter(|e| matches!(e, OwnedEvent::StartBlockquote))
                .count(),
            1
        );
    }

    #[test]
    fn test_streaming_parser_settitle_emitted_once_before_content() {
        let input = b"@settitle My Title\n\n@chapter Hello\n\nBody.\n";
        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(input);
            crate::events::events(&s).map(|e| e.into_owned()).collect()
        };
        let mut streamed: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        for chunk in input.chunks(5) {
            p.feed(chunk);
        }
        p.finish();
        assert_eq!(bulk, streamed);
        assert_eq!(
            streamed
                .iter()
                .filter(|e| matches!(e, OwnedEvent::Title(_)))
                .count(),
            1
        );
        assert!(matches!(streamed[0], OwnedEvent::Title(_)));
    }

    #[test]
    fn test_streaming_parser_mid_utf8_split() {
        let input = "@chapter Héllo wörld\n\nParagraph with émphasis and \u{1F600}.\n"
            .as_bytes()
            .to_vec();
        let bulk: Vec<OwnedEvent> = {
            let s = String::from_utf8_lossy(&input);
            crate::events::events(&s).map(|e| e.into_owned()).collect()
        };
        let mut streamed: Vec<OwnedEvent> = Vec::new();
        let mut p = StreamingParser::new(|ev| streamed.push(ev));
        // Feed one byte at a time to force splits mid-multibyte-character.
        for b in &input {
            p.feed(std::slice::from_ref(b));
        }
        p.finish();
        assert_eq!(bulk, streamed);
    }
}
