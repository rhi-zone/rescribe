//! Streaming Haddock writer — converts a stream of events to Haddock text.
//!
//! Every Haddock construct's opening markup is knowable from its `Start*`
//! event alone (`=` heading markers, `* `/`(n) ` list-item markers, `[` for a
//! definition term, `> ` for a code line or blockquote, `@key` for a
//! property) — no construct has a prefix that depends on content not yet
//! seen. The one piece of state that can't be written immediately is
//! `Property`'s separating space before its description (only needed *if*
//! the description turns out non-empty), handled with a single `bool` flag
//! on the frame rather than any buffering. `CodeBlock`/`AtCodeBlock`/
//! `DocTest` also arrive as single self-contained events (the whole content
//! string already in hand). So `write_event` writes straight into the shared
//! output buffer as events arrive, holding only O(nesting depth) frame-stack
//! state; `finish` just flushes.
//!
//! # Example
//! ```no_run
//! use haddock_fmt::writer::Writer;
//! use haddock_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartHeading { level: 1 });
//! w.write_event(OwnedEvent::Text("Hello".to_string().into()));
//! w.write_event(OwnedEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming Haddock writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// construct is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to flush any remainder and recover the sink.
pub struct Writer<W: Write> {
    sink: W,
    /// Shared output buffer. Every construct writes here directly; cleared
    /// (capacity retained) after each top-level block is flushed.
    out: String,
    /// Frame stack for the block/inline construct currently being assembled.
    /// Empty at top level — closing a construct with an empty stack flushes.
    stack: Vec<Frame>,
}

enum Frame {
    Paragraph,
    Heading,
    UnorderedList,
    OrderedList {
        num: u32,
    },
    ListItem,
    DefinitionList,
    DefinitionTerm,
    DefinitionDesc,
    Blockquote,
    /// `desc_started` tracks whether the one-space separator before the
    /// description has been written yet — written lazily on the first
    /// inline content event so an empty description costs nothing, exactly
    /// mirroring `build_block`'s `if !description.is_empty() { write(" ") }`.
    Property {
        desc_started: bool,
    },
    Strong,
    Emphasis,
    /// Pushed for the span between `StartLink`/`EndLink`. `events()` emits a
    /// redundant `Text(text)` child in that span (see `events.rs`'s
    /// `Inline::Link` expansion) even though `emit::build_inline` never
    /// reads it back — the whole `"text"<url>` is written immediately at
    /// `StartLink` from its fields, so this frame exists only to make
    /// `accepts_inline` false while it's open, suppressing that redundant
    /// child rather than writing it a second time.
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
        }
    }

    /// Feed one event to the writer. Writes bytes to the sink immediately
    /// whenever this event completes a top-level construct.
    pub fn write_event(&mut self, event: OwnedEvent) {
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

    /// Whether the current innermost frame accepts inline text/markup
    /// contributions (mirrors the old builder's `push_inline` context walk —
    /// every context that walk matched, matched here too).
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph
                    | Frame::Heading
                    | Frame::ListItem
                    | Frame::DefinitionTerm
                    | Frame::DefinitionDesc
                    | Frame::Blockquote
                    | Frame::Property { .. }
                    | Frame::Strong
                    | Frame::Emphasis
            )
        )
    }

    /// Called immediately before writing any inline leaf content or opening
    /// an inline span, so `Property`'s lazy description-separator space gets
    /// written exactly once, right before the first real content.
    fn before_inline_content(&mut self) {
        if let Some(Frame::Property { desc_started }) = self.stack.last_mut()
            && !*desc_started
        {
            *desc_started = true;
            self.out.push(' ');
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
                for _ in 0..level {
                    self.out.push('=');
                }
                self.out.push(' ');
                self.stack.push(Frame::Heading);
            }
            OwnedEvent::EndHeading => {
                if matches!(self.stack.last(), Some(Frame::Heading)) {
                    self.stack.pop();
                    self.out.push_str("\n\n");
                }
            }
            OwnedEvent::CodeBlock { content } => {
                for line in content.lines() {
                    self.out.push_str("> ");
                    self.out.push_str(line);
                    self.out.push('\n');
                }
                self.out.push('\n');
            }
            OwnedEvent::AtCodeBlock { content } => {
                self.out.push_str("@\n");
                self.out.push_str(&content);
                self.out.push_str("\n@\n\n");
            }
            OwnedEvent::StartUnorderedList => self.stack.push(Frame::UnorderedList),
            OwnedEvent::EndUnorderedList => {
                if matches!(self.stack.last(), Some(Frame::UnorderedList)) {
                    self.stack.pop();
                    self.out.push('\n');
                }
            }
            OwnedEvent::StartOrderedList => self.stack.push(Frame::OrderedList { num: 1 }),
            OwnedEvent::EndOrderedList => {
                if matches!(self.stack.last(), Some(Frame::OrderedList { .. })) {
                    self.stack.pop();
                    self.out.push('\n');
                }
            }
            OwnedEvent::StartListItem => {
                match self.stack.last_mut() {
                    Some(Frame::UnorderedList) => self.out.push_str("* "),
                    Some(Frame::OrderedList { num }) => {
                        self.out.push('(');
                        self.out.push_str(&num.to_string());
                        self.out.push_str(") ");
                        *num += 1;
                    }
                    _ => {}
                }
                self.stack.push(Frame::ListItem);
            }
            OwnedEvent::EndListItem => {
                if matches!(self.stack.last(), Some(Frame::ListItem)) {
                    self.stack.pop();
                    self.out.push('\n');
                }
            }
            OwnedEvent::StartDefinitionList => self.stack.push(Frame::DefinitionList),
            OwnedEvent::EndDefinitionList => {
                if matches!(self.stack.last(), Some(Frame::DefinitionList)) {
                    self.stack.pop();
                    self.out.push('\n');
                }
            }
            OwnedEvent::StartDefinitionTerm => {
                self.out.push('[');
                self.stack.push(Frame::DefinitionTerm);
            }
            OwnedEvent::EndDefinitionTerm => {
                if matches!(self.stack.last(), Some(Frame::DefinitionTerm)) {
                    self.stack.pop();
                    self.out.push_str("] ");
                }
            }
            OwnedEvent::StartDefinitionDesc => self.stack.push(Frame::DefinitionDesc),
            OwnedEvent::EndDefinitionDesc => {
                if matches!(self.stack.last(), Some(Frame::DefinitionDesc)) {
                    self.stack.pop();
                    self.out.push('\n');
                }
            }
            OwnedEvent::DocTest { expression, result } => {
                self.out.push_str(">>> ");
                self.out.push_str(&expression);
                self.out.push('\n');
                if let Some(r) = result {
                    self.out.push_str(&r);
                    self.out.push('\n');
                }
                self.out.push('\n');
            }
            OwnedEvent::StartBlockquote => {
                self.out.push_str("> ");
                self.stack.push(Frame::Blockquote);
            }
            OwnedEvent::EndBlockquote => {
                if matches!(self.stack.last(), Some(Frame::Blockquote)) {
                    self.stack.pop();
                    self.out.push_str("\n\n");
                }
            }
            OwnedEvent::Property { key, name } => {
                self.out.push('@');
                self.out.push_str(&key);
                if let Some(n) = name {
                    self.out.push(' ');
                    self.out.push_str(&n);
                }
                self.stack.push(Frame::Property {
                    desc_started: false,
                });
            }
            OwnedEvent::EndProperty => {
                if matches!(self.stack.last(), Some(Frame::Property { .. })) {
                    self.stack.pop();
                    self.out.push_str("\n\n");
                }
            }

            // Inline events
            OwnedEvent::Text(cow) => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push_str(&cow);
                }
            }
            OwnedEvent::InlineCode(cow) => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push('@');
                    self.out.push_str(&cow);
                    self.out.push('@');
                }
            }
            OwnedEvent::StartStrong => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push_str("__");
                }
                self.stack.push(Frame::Strong);
            }
            OwnedEvent::EndStrong => {
                if matches!(self.stack.last(), Some(Frame::Strong)) {
                    self.stack.pop();
                    self.out.push_str("__");
                }
            }
            OwnedEvent::StartEmphasis => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push('/');
                }
                self.stack.push(Frame::Emphasis);
            }
            OwnedEvent::EndEmphasis => {
                if matches!(self.stack.last(), Some(Frame::Emphasis)) {
                    self.stack.pop();
                    self.out.push('/');
                }
            }
            // url/text are fully known at `StartLink`, so the whole
            // `"text"<url>` is written immediately here; the `Frame::Link`
            // pushed afterward exists only to suppress `events()`'s
            // redundant `Text(text)` child (see `Frame::Link`'s doc comment)
            // until `EndLink` pops it.
            OwnedEvent::StartLink { url, text } => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push('"');
                    self.out.push_str(&text);
                    self.out.push_str("\"<");
                    self.out.push_str(&url);
                    self.out.push('>');
                }
                self.stack.push(Frame::Link);
            }
            OwnedEvent::EndLink => {
                if matches!(self.stack.last(), Some(Frame::Link)) {
                    self.stack.pop();
                }
            }
            OwnedEvent::ModuleLink { module } => {
                if self.accepts_inline() {
                    self.before_inline_content();
                    self.out.push('"');
                    self.out.push_str(&module);
                    self.out.push('"');
                }
            }
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
        w.write_event(OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(OwnedEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("= Hello"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text("World".to_string().into()));
        w.write_event(OwnedEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("World"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "= Hello\n\nA paragraph with __bold__ text.\n\n* one\n* two\n\n";
        let evts: Vec<_> = crate::events::events(input)
            .map(|e| e.into_owned())
            .collect();
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
    /// tree-based `emit::build` for the same document.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "= Hello\n\nA paragraph.\n",
            "== Sub\n\nWith __strong__, /emphasis/, @code@, \"link\"<http://x/>, \"Data.List\".\n",
            "* one\n* two\n\n",
            "(1) first\n(2) second\n\n",
            "[term1] definition one\n[term2] definition two\n\n",
            "> a code line\n> another\n\n",
            "@\nraw at-block content\n@\n\n",
            "@since 1.0 Added in this version\n\n",
            "@since 1.0\n\n",
            ">>> 1 + 1\n2\n\n",
            "> a blockquote line\n\n",
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

    /// Regression guard against reintroducing per-event `Block`/`Inline`
    /// subtree reconstruction.
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
                evs.push(OwnedEvent::StartStrong);
                evs.push(OwnedEvent::Text(Cow::Owned("bold".to_string())));
                evs.push(OwnedEvent::EndStrong);
                evs.push(OwnedEvent::EndParagraph);
                evs.push(OwnedEvent::StartUnorderedList);
                for j in 0..2 {
                    evs.push(OwnedEvent::StartListItem);
                    evs.push(OwnedEvent::Text(Cow::Owned(format!("item {j}"))));
                    evs.push(OwnedEvent::EndListItem);
                }
                evs.push(OwnedEvent::EndUnorderedList);
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

    /// Peak memory must stay roughly constant as document size grows (a
    /// peak-to-peak ratio between a small and large run, not an absolute
    /// byte ceiling — the same reasoning as
    /// `test_writer_no_subtree_reconstruction_blowup`: this test binary's
    /// threads share one process-wide `#[global_allocator]`, so an absolute
    /// threshold isn't robust to unrelated concurrent tests' allocation
    /// noise, but a genuine unbounded-growth bug still shows up as a large
    /// ratio between the two runs).
    #[test]
    fn test_writer_peak_memory_bounded() {
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
                    "paragraph number {i} with some filler text for realistic size"
                ))));
                w.write_event(OwnedEvent::EndParagraph);
            }
            w.finish();
            PEAK_BYTES.load(Ordering::SeqCst).saturating_sub(before)
        }

        let small = run_peak(500).max(1);
        let large = run_peak(5000);

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
