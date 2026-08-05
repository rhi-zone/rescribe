//! Chunk-driven (batch) POD parser.
//!
//! Feed input in arbitrarily-sized chunks with [`StreamingParser::feed`], then
//! call [`StreamingParser::finish`] to deliver all events to the handler.
//!
//! # Memory model
//!
//! [`StreamingParser`] mirrors [`crate::parse::parse`]'s own line-oriented
//! block dispatch (headings, `=over`/`=item`/`=back` lists, `=begin`/`=end`
//! regions, verbatim/indented paragraphs, ordinary paragraphs) and flushes a
//! block to the handler as soon as its boundary is confirmed. Memory is
//! O(largest block), not O(full input): [`BatchParser`] and [`BatchSink`]
//! remain the simple buffer-everything alternatives for callers who want the
//! full `PodDoc` or don't care about incrementality.
//!
//! # Example — AST style
//! ```no_run
//! use pod_fmt::batch::BatchParser;
//!
//! let mut p = BatchParser::new();
//! p.feed(b"=head1 Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! let (doc, _diags) = p.finish();
//! ```
//!
//! # Example — event callback style
//! ```no_run
//! use pod_fmt::batch::{StreamingParser, Handler};
//! use pod_fmt::OwnedEvent;
//!
//! let mut events = Vec::new();
//! let mut p = StreamingParser::new(|ev: OwnedEvent| events.push(ev));
//! p.feed(b"=head1 Hello\n\n");
//! p.feed(b"A paragraph.\n");
//! p.finish();
//! ```

use crate::ast::{Diagnostic, PodDoc};
use crate::events::OwnedEvent;
pub use rescribe_format_api::Handler;

/// Chunk-driven POD parser that returns the full AST on finish.
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
    pub fn finish(self) -> (PodDoc, Vec<Diagnostic>) {
        let s = String::from_utf8_lossy(&self.buf);
        crate::parse::parse(&s)
    }
}

/// Which kind of block [`StreamingParser`] is currently accumulating lines
/// for. `Idle` means we're between blocks — the next non-blank line decides.
enum State {
    /// Between blocks. The next line's content decides what comes next,
    /// exactly mirroring `parse::Parser::parse_blocks`'s top-level dispatch.
    Idle,
    /// An ordinary paragraph: ends on a blank line, a command line, or an
    /// indented (verbatim-start) line — mirrors `parse_paragraph`.
    Paragraph,
    /// A verbatim (indented) block: continues through blank lines too, only
    /// ends on a non-blank, non-indented line — mirrors `parse_verbatim`.
    Verbatim,
    /// An `=over`...`=back` list, tracked by nesting depth (incremented on
    /// each nested `=over`, decremented on each `=back`) — mirrors
    /// `parse_list`'s recursive handling of nested `=over` blocks. A `=cut`
    /// line at any depth unwinds the whole list immediately, matching
    /// `parse_list`'s own unconditional `=cut` break propagating up through
    /// every recursion level.
    List { depth: usize },
    /// A `=begin FORMAT`...`=end FORMAT` raw region — mirrors
    /// `parse_begin_end`, which does not match the format identifier and is
    /// not `=cut`-aware (raw content is never POD-interpreted).
    BeginEnd,
}

/// Chunked streaming POD parser that delivers events to a [`Handler`].
///
/// Memory: O(largest block). Blocks are flushed to the handler as soon as
/// their boundary is confirmed by the same line-oriented rules
/// [`crate::parse::parse`] uses; only the current incomplete trailing block
/// (plus a possibly-partial final line) is buffered at any time.
///
/// A POD document has one piece of state that spans blocks: whether we're
/// currently "in POD mode" (`in_pod`, set by `=pod`/`=head*`/`=over`, cleared
/// by `=cut`). Content lines outside POD mode are dropped entirely, exactly
/// as `parse::parse` drops them — this parser tracks the same flag across
/// `feed()` calls so a paragraph or verbatim block that opens under POD mode
/// established by an earlier flushed block is still recognized (re-parsed
/// standalone with a synthetic leading `=pod` line to establish the same
/// state in the isolated re-parse).
pub struct StreamingParser<H: Handler<OwnedEvent>> {
    handler: H,
    line_buf: Vec<u8>,
    block_lines: Vec<String>,
    state: State,
    in_pod: bool,
}

impl<H: Handler<OwnedEvent>> StreamingParser<H> {
    /// Create a new `StreamingParser` that delivers events to `handler`.
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            line_buf: Vec::new(),
            block_lines: Vec::new(),
            state: State::Idle,
            in_pod: false,
        }
    }

    /// Feed a chunk of bytes. May call `handler.handle()` zero or more
    /// times. Handles chunk boundaries that split a line (including
    /// mid-UTF-8-character splits — a line's bytes are only decoded once
    /// the full line has arrived) and CRLF line endings.
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
        match &self.state {
            State::List { depth } => {
                let depth = *depth;
                let trimmed = line.trim_start();
                if trimmed.starts_with("=cut") {
                    self.flush_block(false);
                    self.state = State::Idle;
                    self.feed_line(line);
                    return;
                }
                if trimmed.starts_with("=over") {
                    self.block_lines.push(line);
                    self.state = State::List { depth: depth + 1 };
                    return;
                }
                if trimmed.starts_with("=back") {
                    self.block_lines.push(line);
                    if depth <= 1 {
                        self.flush_block(false);
                        self.state = State::Idle;
                    } else {
                        self.state = State::List { depth: depth - 1 };
                    }
                    return;
                }
                self.block_lines.push(line);
            }

            State::BeginEnd => {
                let ends = line.starts_with("=end");
                self.block_lines.push(line);
                if ends {
                    self.flush_block(false);
                    self.state = State::Idle;
                }
            }

            State::Verbatim => {
                let is_blank = line.is_empty();
                let is_indented = line.starts_with(' ') || line.starts_with('\t');
                if is_blank || is_indented {
                    self.block_lines.push(line);
                    return;
                }
                self.flush_block(true);
                self.state = State::Idle;
                self.feed_line(line);
            }

            State::Paragraph => {
                let is_blank = line.trim().is_empty();
                let is_command = line.starts_with('=');
                let is_indented = line.starts_with(' ') || line.starts_with('\t');
                if is_blank {
                    self.flush_block(true);
                    self.state = State::Idle;
                    return;
                }
                if is_command || is_indented {
                    self.flush_block(true);
                    self.state = State::Idle;
                    self.feed_line(line);
                    return;
                }
                self.block_lines.push(line);
            }

            State::Idle => self.feed_line_idle(line),
        }
    }

    /// Top-level block dispatch, mirroring `parse::Parser::parse_blocks`'s
    /// per-line checks in the same order.
    fn feed_line_idle(&mut self, line: String) {
        if line.starts_with("=pod") || line.starts_with("=head") || line.starts_with("=over") {
            self.in_pod = true;
        }
        if line.starts_with("=cut") {
            self.in_pod = false;
            return;
        }
        if !self.in_pod && !line.starts_with('=') {
            return;
        }

        if line.starts_with('=') {
            if line.starts_with("=over") {
                self.block_lines.clear();
                self.block_lines.push(line);
                self.state = State::List { depth: 1 };
                return;
            }
            if line.starts_with("=pod") {
                // Marker only, no block.
                return;
            }
            if line.starts_with("=begin") {
                self.block_lines.clear();
                self.block_lines.push(line);
                self.state = State::BeginEnd;
                return;
            }
            // =head*, =for, =encoding, and stray =end/=back/=item/unknown
            // commands are all single-line blocks.
            self.flush_single_line(&line);
            return;
        }

        if line.trim().is_empty() {
            return;
        }

        if line.starts_with(' ') || line.starts_with('\t') {
            self.block_lines.clear();
            self.block_lines.push(line);
            self.state = State::Verbatim;
            return;
        }

        self.block_lines.clear();
        self.block_lines.push(line);
        self.state = State::Paragraph;
    }

    /// Re-parse a single already-complete command line and forward any
    /// events it produces.
    fn flush_single_line(&mut self, line: &str) {
        let (doc, _diags) = crate::parse::parse(line);
        for event in crate::events::EventIter::new(&doc) {
            self.handler.handle(event.into_owned());
        }
    }

    /// Re-parse the accumulated block and forward its events.
    ///
    /// `needs_pod_prefix` is set for paragraph/verbatim blocks: those blocks
    /// only ever start accumulating while `self.in_pod` is already true (the
    /// `Idle` gate above enforces this), but a standalone re-parse of just
    /// their lines starts with `in_pod = false` and would otherwise drop the
    /// content — the synthetic leading `=pod` line (itself producing no
    /// event) re-establishes the same state for the isolated re-parse.
    /// List/`=begin` blocks start with a `=` line and need no such prefix.
    fn flush_block(&mut self, needs_pod_prefix: bool) {
        if self.block_lines.is_empty() {
            return;
        }
        let joined = self.block_lines.join("\n");
        self.block_lines.clear();
        let text = if needs_pod_prefix {
            format!("=pod\n{joined}")
        } else {
            joined
        };
        let (doc, _diags) = crate::parse::parse(&text);
        for event in crate::events::EventIter::new(&doc) {
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
        match self.state {
            State::List { .. } | State::BeginEnd => self.flush_block(false),
            State::Verbatim | State::Paragraph => self.flush_block(true),
            State::Idle => {}
        }
    }
}

/// Chunk-driven POD parser that delivers events to a callback on finish.
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
        let (doc, _diags) = crate::parse::parse(&s);
        for event in crate::events::EventIter::new(&doc) {
            (self.callback)(event.into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc_probe::{CURRENT_BYTES, PEAK_BYTES};
    use crate::events::OwnedEvent;

    #[test]
    fn test_batch_parser_basic() {
        let mut p = BatchParser::new();
        p.feed(b"=head1 Hello\n\n");
        p.feed(b"A paragraph.\n");
        let (doc, diags) = p.finish();
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_streaming_parser_events() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev: OwnedEvent| evs.push(ev));
        p.feed(b"=head1 Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();
        assert!(
            evs.iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 }))
        );
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::StartParagraph)));
    }

    #[test]
    fn test_batch_sink_events() {
        let mut events = Vec::new();
        let mut sink = BatchSink::new(|ev: OwnedEvent| events.push(ev));
        sink.feed(b"=head1 Hello\n\n");
        sink.feed(b"A paragraph.\n");
        sink.finish();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartHeading { level: 1 }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, OwnedEvent::StartParagraph))
        );
    }

    /// Feed events land in the handler *before* `finish()` — the direct
    /// proof this is not buffer-then-finish. If this were still
    /// buffer-then-finish, `delivered` would be empty here.
    #[test]
    fn test_streaming_parser_delivers_before_finish() {
        let delivered = std::rc::Rc::new(std::cell::RefCell::new(Vec::<OwnedEvent>::new()));
        let d = delivered.clone();
        let mut p = StreamingParser::new(move |ev: OwnedEvent| d.borrow_mut().push(ev));
        p.feed(b"=head1 Hello\n\nA paragraph.\n\n");
        assert!(
            !delivered.borrow().is_empty(),
            "expected events to be delivered by feed() alone, before finish()"
        );
        p.finish();
    }

    fn bulk_events(input: &[u8]) -> Vec<OwnedEvent> {
        let s = String::from_utf8_lossy(input);
        crate::events::EventIter::new(&crate::parse::parse(&s).0)
            .map(|e| e.into_owned())
            .collect()
    }

    fn streamed_events(input: &[u8], chunk_size: usize) -> Vec<OwnedEvent> {
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

    fn assert_matches_bulk(input: &[u8]) {
        let bulk = bulk_events(input);
        for chunk_size in [0, 1, 3, 7, 13] {
            let streamed = streamed_events(input, chunk_size);
            assert_eq!(
                bulk,
                streamed,
                "streamed diverged from bulk at chunk_size={chunk_size} for input:\n{:?}",
                String::from_utf8_lossy(input)
            );
        }
    }

    #[test]
    fn test_streaming_split_heading_and_paragraph() {
        assert_matches_bulk(b"=head1 Hello\n\nA paragraph.\n");
    }

    #[test]
    fn test_streaming_split_list() {
        assert_matches_bulk(b"=over\n\n=item * One\n\n=item * Two\n\n=back\n");
    }

    #[test]
    fn test_streaming_split_nested_list() {
        assert_matches_bulk(
            b"=over\n\n=item * Outer\n\n=over\n\n=item * Inner\n\n=back\n\n=back\n",
        );
    }

    #[test]
    fn test_streaming_split_verbatim_with_blank_line() {
        assert_matches_bulk(b"=pod\n\n    line one\n\n    line two\n\nAfter.\n");
    }

    #[test]
    fn test_streaming_split_begin_end() {
        assert_matches_bulk(b"=begin html\n\n<p>raw</p>\n\n=end html\n");
    }

    #[test]
    fn test_streaming_split_definition_list() {
        assert_matches_bulk(b"term1\n\ndefinition body one.\n\nterm2\n\ndefinition body two.\n");
    }

    #[test]
    fn test_streaming_split_encoding_and_for() {
        assert_matches_bulk(b"=encoding utf8\n\n=for html <br/>\n\n=head1 Hi\n\ntext\n");
    }

    #[test]
    fn test_streaming_split_mid_utf8_char() {
        // "café" — the 'é' is 2 UTF-8 bytes, split across separate feed() calls.
        let input = "=pod\n\ncafé society\n\nAnother caf\u{e9} paragraph.\n".as_bytes();
        assert_matches_bulk(input);
    }

    #[test]
    fn test_streaming_split_pod_mode_carries_across_blocks() {
        // The paragraph after =head1 relies on in_pod carried across blocks.
        assert_matches_bulk(b"=head1 NAME\n\nDescription paragraph.\n\n    verbatim too\n");
    }

    #[test]
    fn test_streaming_split_over_no_back_then_cut() {
        assert_matches_bulk(b"=over\n\n=item * Orphan item\n\n=cut\n");
    }

    #[test]
    fn test_streaming_split_malformed_begin_end() {
        assert_matches_bulk(b"=begin html\n\n<p>Unclosed begin block</p>\n");
    }

    #[test]
    fn test_streaming_split_stray_back_and_item() {
        assert_matches_bulk(b"=pod\n\n=back\n\n=cut\n");
        assert_matches_bulk(b"=item stray\n\nParagraph.\n");
    }

    #[test]
    fn test_streaming_split_bold_and_link() {
        assert_matches_bulk(b"=pod\n\nB<bold> and L<perlpod|http://x/> text.\n");
    }

    /// Build a synthetic multi-block POD document with `n` heading+paragraph
    /// sections, so a genuinely incremental parser has plenty of block
    /// boundaries to flush at well before the whole document is fed.
    fn synthetic_doc(n: usize) -> Vec<u8> {
        let mut s = String::new();
        for i in 0..n {
            s.push_str(&format!(
                "=head1 Section {i}\n\nParagraph {i} with some B<bold> filler text to give \
                 it realistic size and a L<link|http://example.com/{i}>.\n\n"
            ));
        }
        s.into_bytes()
    }

    /// Peak memory must stay roughly *constant* as document size grows, not
    /// scale with it — the direct proof `feed()` flushes each block instead
    /// of buffering the whole input. Uses a peak-to-peak *ratio* between a
    /// small and a large run (10x the sections), the same technique
    /// `writer.rs`'s `test_writer_peak_memory_bounded` uses, rather than an
    /// absolute byte ceiling: `cargo test` runs this binary's tests on
    /// multiple threads sharing one process-wide `#[global_allocator]`
    /// (`crate::alloc_probe`), so unrelated tests' concurrent allocations add
    /// noise an absolute threshold can't distinguish from a real O(document)
    /// regression at the sizes involved here, but a genuine unbounded-growth
    /// bug still shows up as a large ratio between the two runs.
    #[test]
    fn test_streaming_parser_peak_memory_bounded() {
        fn run_peak(n: usize) -> usize {
            let input = synthetic_doc(n);
            PEAK_BYTES.with(|p| p.set(0));
            let before = CURRENT_BYTES.with(|c| c.get());
            let mut evs = 0usize;
            let mut p = StreamingParser::new(|_: OwnedEvent| evs += 1);
            // Feed in small chunks, well smaller than one block, so the
            // parser must actually flush incrementally to stay bounded.
            for chunk in input.chunks(64) {
                p.feed(chunk);
            }
            p.finish();
            std::hint::black_box(evs);
            PEAK_BYTES.with(|p| p.get()).saturating_sub(before)
        }

        let small = run_peak(200).max(1);
        let large = run_peak(2000); // 10x the sections

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 20.0,
            "peak memory did not stay roughly constant across document sizes: \
             {small} bytes peak @200 sections -> {large} bytes peak @2000 sections \
             (ratio {ratio:.2}); this suggests StreamingParser is buffering O(document) \
             instead of O(largest block)"
        );
    }
}
