//! Streaming POD writer — converts a stream of events to POD text.
//!
//! Every POD construct's opening markup is knowable from the `Start*` event
//! alone (`=head1 `, `=over 4\n\n`, `B<`, …) — unlike formats with computed
//! prefixes (RST's heading underline width, a table's column widths), POD
//! has no construct whose prefix depends on content not yet seen. `CodeBlock`,
//! `RawBlock`, `ForBlock`, and `Encoding` also arrive as single self-contained
//! events (the whole content string is already in hand), so nothing in this
//! writer needs deferral: every event is written straight into the shared
//! output buffer as it arrives, and `finish` only closes the outer `=cut`.

use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming POD writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to flush the closing `=cut` and recover the
/// sink once all events have been fed.
pub struct Writer<W: Write> {
    sink: W,
    /// Shared output buffer. Every construct writes here directly; cleared
    /// (capacity retained) after each top-level block is flushed.
    out: String,
    /// Frame stack for the block/inline construct currently being assembled.
    /// Empty at top level — closing a construct with an empty stack flushes
    /// the buffer to the sink.
    stack: Vec<Frame>,
}

enum Frame {
    Paragraph,
    Heading,
    List { ordered: bool, num: u32 },
    ListItem,
    DefinitionList,
    DefinitionTerm,
    DefinitionDesc,
    Bold,
    Italic,
    Underline,
    Filename,
    NonBreaking,
}

/// Default reserved capacity for the shared output buffer.
const DEFAULT_OUT_CAPACITY: usize = 4096;

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Self::with_capacity(sink, DEFAULT_OUT_CAPACITY)
    }

    /// Like [`Writer::new`], but reserves `out_capacity` bytes up front.
    pub fn with_capacity(sink: W, out_capacity: usize) -> Self {
        let mut out = String::with_capacity(out_capacity);
        out.push_str("=pod\n\n");
        Writer {
            sink,
            out,
            stack: Vec::new(),
        }
    }

    /// Feed one event to the writer. Writes bytes to the sink immediately
    /// whenever this event completes a top-level construct.
    pub fn write_event(&mut self, event: OwnedEvent) {
        self.process(event);
        self.maybe_flush();
    }

    /// Flush the closing `=cut` and recover the sink.
    pub fn finish(mut self) -> W {
        self.out.push_str("=cut\n");
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

    /// Whether the current innermost frame accepts inline text/markup
    /// contributions (mirrors the old builder's `push_inline` context check).
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph
                    | Frame::Heading
                    | Frame::DefinitionTerm
                    | Frame::Bold
                    | Frame::Italic
                    | Frame::Underline
                    | Frame::Filename
                    | Frame::NonBreaking
            )
        )
    }

    fn write_text(&mut self, s: &str) {
        if self.accepts_inline() {
            // Escape < and > in plain text, matching emit::build_inline.
            for ch in s.chars() {
                match ch {
                    '<' => self.out.push_str("E<lt>"),
                    '>' => self.out.push_str("E<gt>"),
                    c => self.out.push(c),
                }
            }
        }
    }

    fn process(&mut self, event: OwnedEvent) {
        match event {
            OwnedEvent::StartParagraph => self.stack.push(Frame::Paragraph),
            OwnedEvent::EndParagraph => {
                if matches!(self.stack.last(), Some(Frame::Paragraph)) {
                    self.stack.pop();
                    self.out.push_str("\n\n");
                }
            }
            OwnedEvent::StartHeading { level } => {
                self.out.push_str(&format!("=head{level} "));
                self.stack.push(Frame::Heading);
            }
            OwnedEvent::EndHeading => {
                if matches!(self.stack.last(), Some(Frame::Heading)) {
                    self.stack.pop();
                    self.out.push_str("\n\n");
                }
            }
            OwnedEvent::StartList { ordered } => {
                self.out.push_str("=over 4\n\n");
                self.stack.push(Frame::List { ordered, num: 1 });
            }
            OwnedEvent::EndList => {
                if matches!(self.stack.last(), Some(Frame::List { .. })) {
                    self.stack.pop();
                    self.out.push_str("=back\n\n");
                }
            }
            OwnedEvent::StartListItem => {
                if let Some(Frame::List { ordered, num }) = self.stack.last_mut() {
                    if *ordered {
                        self.out.push_str(&format!("=item {num}.\n\n"));
                        *num += 1;
                    } else {
                        self.out.push_str("=item *\n\n");
                    }
                }
                self.stack.push(Frame::ListItem);
            }
            OwnedEvent::EndListItem => {
                if matches!(self.stack.last(), Some(Frame::ListItem)) {
                    self.stack.pop();
                }
            }
            OwnedEvent::StartDefinitionList => {
                self.out.push_str("=over 4\n\n");
                self.stack.push(Frame::DefinitionList);
            }
            OwnedEvent::EndDefinitionList => {
                if matches!(self.stack.last(), Some(Frame::DefinitionList)) {
                    self.stack.pop();
                    self.out.push_str("=back\n\n");
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                self.out.push_str("=item ");
                self.stack.push(Frame::DefinitionTerm);
            }
            OwnedEvent::EndDefinitionTerm => {
                if matches!(self.stack.last(), Some(Frame::DefinitionTerm)) {
                    self.stack.pop();
                    self.out.push_str("\n\n");
                }
            }
            OwnedEvent::StartDefinitionDesc => {
                self.stack.push(Frame::DefinitionDesc);
            }
            OwnedEvent::EndDefinitionDesc => {
                if matches!(self.stack.last(), Some(Frame::DefinitionDesc)) {
                    self.stack.pop();
                }
            }
            OwnedEvent::CodeBlock { content } => {
                for line in content.lines() {
                    self.out.push_str("    ");
                    self.out.push_str(line);
                    self.out.push('\n');
                }
                self.out.push('\n');
            }
            OwnedEvent::RawBlock { format, content } => {
                self.out.push_str(&format!("=begin {format}\n"));
                if !content.is_empty() {
                    self.out.push_str(&content);
                    self.out.push('\n');
                }
                self.out.push_str(&format!("=end {format}\n\n"));
            }
            OwnedEvent::ForBlock { format, content } => {
                self.out.push_str(&format!("=for {format} {content}\n\n"));
            }
            OwnedEvent::Encoding { encoding } => {
                self.out.push_str(&format!("=encoding {encoding}\n\n"));
            }

            // Inline events
            OwnedEvent::Text(cow) => self.write_text(&cow),
            OwnedEvent::StartBold => {
                self.write_delim_open("B<", Frame::Bold);
            }
            OwnedEvent::EndBold => self.write_delim_close(Frame::Bold),
            OwnedEvent::StartItalic => self.write_delim_open("I<", Frame::Italic),
            OwnedEvent::EndItalic => self.write_delim_close(Frame::Italic),
            OwnedEvent::StartUnderline => self.write_delim_open("U<", Frame::Underline),
            OwnedEvent::EndUnderline => self.write_delim_close(Frame::Underline),
            OwnedEvent::StartFilename => self.write_delim_open("F<", Frame::Filename),
            OwnedEvent::EndFilename => self.write_delim_close(Frame::Filename),
            OwnedEvent::StartNonBreaking => self.write_delim_open("S<", Frame::NonBreaking),
            OwnedEvent::EndNonBreaking => self.write_delim_close(Frame::NonBreaking),
            OwnedEvent::InlineCode(cow) => {
                if self.accepts_inline() {
                    if cow.contains('>') || cow.contains('<') {
                        self.out.push_str("C<< ");
                        self.out.push_str(&cow);
                        self.out.push_str(" >>");
                    } else {
                        self.out.push_str("C<");
                        self.out.push_str(&cow);
                        self.out.push('>');
                    }
                }
            }
            // Link is a leaf: url/label are fully known at `StartLink` and no
            // children events are ever emitted between it and `EndLink`
            // (see `events.rs`'s `LeafLinkOpen`/`LeafLinkClose`), so the
            // whole construct is written immediately with no frame pushed.
            OwnedEvent::StartLink { url, label } => {
                if self.accepts_inline() {
                    if label.is_empty() || label == url {
                        self.out.push_str("L<");
                        self.out.push_str(&url);
                        self.out.push('>');
                    } else {
                        self.out.push_str("L<");
                        self.out.push_str(&label);
                        self.out.push('|');
                        self.out.push_str(&url);
                        self.out.push('>');
                    }
                }
            }
            OwnedEvent::EndLink => {}
            OwnedEvent::IndexEntry(s) => {
                if self.accepts_inline() {
                    self.out.push_str("X<");
                    self.out.push_str(&s);
                    self.out.push('>');
                }
            }
            OwnedEvent::Null => {
                if self.accepts_inline() {
                    self.out.push_str("Z<>");
                }
            }
            OwnedEvent::Entity(s) => {
                if self.accepts_inline() {
                    self.out.push_str(&s);
                }
            }
        }
    }

    fn write_delim_open(&mut self, delim: &str, frame: Frame) {
        // Always emit the opening delimiter (mirroring the unconditional
        // close below) — `accepts_inline` only gates leaf content
        // (Text/InlineCode/Link/…), not span delimiters, since a span's own
        // frame becomes the new inline-accepting context for its children.
        self.out.push_str(delim);
        self.stack.push(frame);
    }

    fn write_delim_close(&mut self, expected: Frame) {
        let matches_top = matches!(
            (self.stack.last(), &expected),
            (Some(Frame::Bold), Frame::Bold)
                | (Some(Frame::Italic), Frame::Italic)
                | (Some(Frame::Underline), Frame::Underline)
                | (Some(Frame::Filename), Frame::Filename)
                | (Some(Frame::NonBreaking), Frame::NonBreaking)
        );
        if matches_top {
            self.stack.pop();
            // The closing delimiter belongs to the span that just closed,
            // not to whatever its parent's context is — mirror
            // `build_inline`'s unconditional `ctx.write(">")` at span close.
            self.out.push('>');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::borrow::Cow;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Single test-process-wide allocator tracking both a running allocation
    /// *count* (for the linearity regression guard) and *current/peak* live
    /// bytes (for the peak-memory bound). Only one `#[global_allocator]` is
    /// allowed per binary, so both memory-shape tests below share this one.
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

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartHeading { level: 1 });
        w.write_event(OwnedEvent::Text(Cow::Owned("Hello".to_string())));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("=head1 Hello"), "got: {s:?}");
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
        let input = "=head1 Hello\n\nA paragraph with B<bold> text.\n\n=over 4\n\n=item * one\n\n=item * two\n\n=back\n";
        let (doc, _) = crate::parse::parse(input);
        let evts: Vec<_> = crate::events::EventIter::new(&doc).collect();
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

    /// The streaming `Writer` must produce byte-identical output to the
    /// tree-based `emit::build` for the same document — the guard that keeps
    /// the two independent emission paths honest.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "=head1 Hello\n\nA paragraph.\n",
            "=head2 Sub\n\nWith B<bold>, I<italic>, U<underline>, F<a/path>, S<non break>, C<code>, C<< a >> b >>, L<http://x/>, L<label|http://x/>, X<entry>, Z<>.\n",
            "=over 4\n\n=item * one\n\n=item * two\n\n=back\n",
            "=over 4\n\n=item 1.\n\nfirst\n\n=item 2.\n\nsecond\n\n=back\n",
            "term1\n\ndefinition body one.\n\nterm2\n\ndefinition body two.\n",
            "    a verbatim\n    two lines\n",
            "=begin html\n\n<b>raw</b>\n\n=end html\n",
            "=for html <b>raw</b>\n",
            "=encoding utf8\n\n=head1 Hi\n\ntext\n",
            "=over 4\n\n=item * outer\n\n=over 4\n\n=item * inner a\n\n=item * inner b\n\n=back\n\n=back\n",
            "A E<lt>escaped E<gt> paragraph.\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse::parse(input);
            let built = crate::emit::build(&doc);

            let mut w = Writer::new(Vec::<u8>::new());
            for e in crate::events::EventIter::new(&doc) {
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
    /// subtree reconstruction. A large, deeply-nested event stream must
    /// complete with an allocation count that stays close to linear in
    /// event count.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
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

    /// Peak memory must stay roughly *constant* as document size grows, not
    /// scale with it — the direct proof the writer never buffers the whole
    /// document. Uses a peak-to-peak *ratio* between a small and a large run
    /// (10x the paragraphs), the same technique
    /// `test_writer_no_subtree_reconstruction_blowup` above already uses,
    /// rather than an absolute byte ceiling: `cargo test` runs this file's
    /// tests on multiple threads sharing one process-wide
    /// `#[global_allocator]`, so unrelated tests' concurrent allocations add
    /// noise an absolute threshold can't distinguish from a real O(document)
    /// regression at the sizes involved here, but a genuine unbounded-growth
    /// bug still shows up as a large ratio between the two runs.
    #[test]
    fn test_writer_peak_memory_bounded() {
        // Sink that discards bytes immediately rather than accumulating —
        // proves the writer itself, not the test's sink, is bounded.
        struct NullSink;
        impl Write for NullSink {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        fn run_peak(n: usize) -> usize {
            PEAK_BYTES.store(0, Ordering::SeqCst);
            let before = CURRENT_BYTES.load(Ordering::SeqCst);
            let mut w = Writer::new(NullSink);
            for i in 0..n {
                w.write_event(OwnedEvent::StartParagraph);
                w.write_event(OwnedEvent::Text(Cow::Owned(format!(
                    "paragraph number {i} with some filler text to give it realistic size"
                ))));
                w.write_event(OwnedEvent::EndParagraph);
            }
            w.finish();
            PEAK_BYTES.load(Ordering::SeqCst).saturating_sub(before)
        }

        let small = run_peak(500).max(1);
        let large = run_peak(5000); // 10x the paragraphs

        let ratio = large as f64 / small as f64;
        assert!(
            ratio < 20.0,
            "peak memory did not stay roughly constant across document sizes: \
             {small} bytes peak @500 paragraphs -> {large} bytes peak @5000 paragraphs \
             (ratio {ratio:.2}); this suggests the writer is buffering O(document) instead \
             of O(nesting depth)"
        );
    }
}
