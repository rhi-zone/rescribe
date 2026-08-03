//! Streaming man page writer — converts a stream of events directly to man
//! page text.
//!
//! # Memory model
//!
//! [`Writer`] never constructs a [`crate::ast::Block`]/[`crate::ast::Inline`]
//! value and never calls [`crate::emit::build`]. It is a second, independent
//! emission path from the tree-based `build()` function, not a thin wrapper
//! around it — the same shape as `t2t-fmt`'s and `fountain-fmt`'s streaming
//! writers.
//!
//! # Construct classification
//!
//! Reading `emit.rs` end to end, almost every construct is
//! write-straight-through into a single shared `out: String` buffer, using
//! a frame stack of small marks/scalars (never accumulated subtree content)
//! to know what to write at each event, mirroring `rst-fmt`/`t2t-fmt`.
//!
//! Two constructs need bounded buffering:
//!
//! - **The `.TH` title-header line** needs its five fields (title, section,
//!   date, source, manual) before it can be written, and it must be the
//!   very first thing in the output. `events()`'s own contract (see
//!   `events.rs`) emits a dedicated `Event::Metadata` right after
//!   `StartDocument` carrying exactly those five fields — O(field count)
//!   buffering, not O(document size).
//! - **Headings** (`.SH`/`.SS`) render their title with `extract_text()`,
//!   *not* `build_inlines()`: `emit.rs`'s `Block::Heading` arm flattens all
//!   inline markup (bold/italic/superscript/subscript/link wrappers and
//!   escaping are all dropped — only raw text and code content survive,
//!   uppercased) — a heading's own text must be assembled before the
//!   uppercased `.SH`/`.SS` line can be written. This is O(heading text
//!   length), i.e. one nesting frame, not O(document size).
//!
//! Every other block (paragraph, indented paragraph, code block, example
//! block, list, definition list, horizontal rule, comment) and every other
//! inline (bold, italic, code, superscript, subscript, link) writes directly
//! into `out` as its events arrive, exactly mirroring `emit.rs`'s recursive
//! `build_block`/`build_inline` output byte-for-byte.
//!
//! `emit.rs`'s `Block::List`/`Block::DefinitionList` arms give a `Paragraph`
//! child different framing depending on its *parent's type* (not position):
//! a `Paragraph` directly inside a list item or a definition description is
//! written "bare" (no `.PP\n` marker, just its inlines + a single `\n`); a
//! `Paragraph` anywhere else gets the full `.PP\n` + inlines + `\n` form.
//! This is decided at `StartParagraph`/`EndParagraph` purely by inspecting
//! the parent frame already on the stack — the same "known at open, applied
//! at close" shape `t2t-fmt`'s writer uses for its own blockquote/list-item
//! paragraph framing.
//!
//! Each top-level block is flushed to the sink and `out` is cleared (keeping
//! its capacity) as soon as the frame stack empties, mirroring `rst-fmt`/
//! `t2t-fmt`.
//!
//! # Example
//! ```no_run
//! use man_fmt::writer::Writer;
//! use man_fmt::OwnedManEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedManEvent::StartHeading { level: 2 });
//! w.write_event(OwnedManEvent::Text("NAME".to_string().into()));
//! w.write_event(OwnedManEvent::EndHeading);
//! let bytes = w.finish();
//! ```

use crate::emit::escape_man;
use crate::events::OwnedManEvent as Event;
use std::io::Write;

/// Default capacity reserved for `Writer::out`, mirroring `rst-fmt`'s
/// `DEFAULT_OUT_CAPACITY`.
const DEFAULT_OUT_CAPACITY: usize = 4096;

/// Streaming man page writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// block is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to recover the sink once all events have been
/// fed.
pub struct Writer<W: Write> {
    sink: W,
    /// The single shared output buffer. Cleared (capacity retained) after
    /// each top-level block is flushed.
    out: String,
    /// Frame stack for the block/inline construct currently being
    /// assembled. Empty at top level.
    stack: Vec<Frame>,
    /// Whether the `.TH` line has been written yet.
    th_written: bool,
}

/// Frames carry only a mark into the shared buffer and tiny scalars — never
/// accumulated subtree content — except `Heading`, whose flattened text
/// (see the module doc) is bounded by one heading's own text length.
enum Frame {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph {
        mark: usize,
    },
    IndentedParagraph {
        mark: usize,
    },
    List {
        mark: usize,
        ordered: bool,
        item_index: usize,
    },
    ListItem {
        mark: usize,
    },
    DefinitionList,
    DefinitionTerm {
        mark: usize,
    },
    DefinitionDesc {
        mark: usize,
    },
    /// Any inline span whose closing text is a fixed string (bold, italic,
    /// superscript, subscript — all symmetric open/close markers).
    Inline {
        mark: usize,
        close: &'static str,
    },
    /// `children (url)` — man-fmt writes the URL *after* any link text, so
    /// it's carried on the frame and written at `EndLink`, not `StartLink`.
    Link {
        mark: usize,
        url: String,
    },
}

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
            th_written: false,
        }
    }

    /// Feed one event to the writer. May write bytes to the sink immediately
    /// if this event completes a top-level block.
    pub fn write_event(&mut self, event: Event) {
        match event {
            Event::StartDocument => return,
            Event::Metadata {
                title,
                section,
                date,
                source,
                manual,
            } => {
                self.write_th(
                    title.as_deref(),
                    section.as_deref(),
                    date.as_deref(),
                    source.as_deref(),
                    manual.as_deref(),
                );
                return;
            }
            _ => {}
        }
        if !self.th_written {
            // A block event arrived before Metadata (violating events()'s
            // documented ordering) — build()'s own .TH line is
            // unconditional, so emit its all-defaults form before
            // proceeding.
            self.write_th(None, None, None, None, None);
        }
        self.process(event);
        self.maybe_flush();
    }

    /// Recover the underlying sink. Does not write anything itself — every
    /// completed top-level block was already flushed by `write_event`.
    pub fn finish(mut self) -> W {
        if !self.th_written {
            // Zero events fed (or only StartDocument): build()'s own .TH
            // line is unconditional, so an empty document still gets one.
            self.write_th(None, None, None, None, None);
        }
        self.flush();
        self.sink
    }

    fn write_th(
        &mut self,
        title: Option<&str>,
        section: Option<&str>,
        date: Option<&str>,
        source: Option<&str>,
        manual: Option<&str>,
    ) {
        self.out.push_str(".TH ");
        self.out
            .push_str(&title.unwrap_or("UNTITLED").to_uppercase());
        self.out.push(' ');
        self.out.push_str(section.unwrap_or("1"));
        self.out.push_str(" \"");
        self.out.push_str(date.unwrap_or(""));
        self.out.push_str("\" \"");
        self.out.push_str(source.unwrap_or(""));
        self.out.push_str("\" \"");
        self.out.push_str(manual.unwrap_or(""));
        self.out.push_str("\"\n");
        self.th_written = true;
        self.maybe_flush();
    }

    // ── Buffer primitives ─────────────────────────────────────────────────

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

    /// Mirrors `emit.rs`'s `BuildContext::newline`: ensure `out` ends with a
    /// newline before writing a new block's marker.
    ///
    /// `out` is cleared after every top-level block flushes (see
    /// `maybe_flush`), so an *empty* buffer here doesn't mean "start of
    /// output, no newline yet" — every top-level block's own close event
    /// writes a trailing `\n` before the flush that empties `out` (verified
    /// across every `EndX` arm in `process`), so an empty buffer is always
    /// logically preceded by a newline. Treat empty the same as
    /// already-ends-with-newline; only a genuinely non-newline-terminated
    /// buffer needs one appended.
    fn newline(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
    }

    /// Whether the top-of-stack frame accepts block children (mirrors
    /// `emit.rs`'s implicit contract: only the document root, a list item,
    /// or a definition description ever collect nested blocks — the same
    /// set `collect_doc_from_events`'s `push_block` accepts).
    fn accepts_blocks(&self) -> bool {
        matches!(
            self.stack.last(),
            None | Some(Frame::ListItem { .. } | Frame::DefinitionDesc { .. })
        )
    }

    /// Whether the top-of-stack frame accepts inline children written
    /// through the normal (non-flattened) path.
    fn accepts_inline(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                Frame::Paragraph { .. }
                    | Frame::IndentedParagraph { .. }
                    | Frame::DefinitionTerm { .. }
                    | Frame::Inline { .. }
                    | Frame::Link { .. }
            )
        )
    }

    /// Whether inline events should be flattened into the innermost
    /// `Heading`'s raw text buffer instead of written normally — true
    /// whenever the top of the stack is a `Heading` frame, since headings
    /// never push a nested `Inline`/`Link` frame for their own inline
    /// container children (see the module doc: `extract_text` drops all
    /// markup, so there is nothing for those containers to write).
    fn in_heading(&self) -> bool {
        matches!(self.stack.last(), Some(Frame::Heading { .. }))
    }

    fn open_inline_span(&mut self, open: &str, close: &'static str) {
        let mark = self.out.len();
        self.out.push_str(open);
        self.stack.push(Frame::Inline { mark, close });
    }

    fn close_inline_span(&mut self) {
        if let Some(Frame::Inline { mark, close }) = self.stack.pop() {
            self.out.push_str(close);
            if !self.accepts_inline() {
                self.out.truncate(mark);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process(&mut self, event: Event) {
        match event {
            Event::StartDocument | Event::EndDocument => {}
            Event::Metadata { .. } => {
                // Only meaningful as the event right after StartDocument,
                // handled in write_event() before reaching process(); a
                // Metadata arriving later (violating events()'s documented
                // ordering) has no well-defined header slot left and is
                // dropped.
            }

            // ── Block open/close ────────────────────────────────────────
            Event::StartParagraph => {
                let mark = self.out.len();
                let bare = matches!(
                    self.stack.last(),
                    Some(Frame::ListItem { .. } | Frame::DefinitionDesc { .. })
                );
                if !bare {
                    self.newline();
                    self.out.push_str(".PP\n");
                }
                self.stack.push(Frame::Paragraph { mark });
            }
            Event::EndParagraph => {
                if let Some(Frame::Paragraph { mark }) = self.stack.pop() {
                    self.out.push('\n');
                    if !self.accepts_blocks() {
                        self.out.truncate(mark);
                    } else if self.stack.is_empty() {
                        self.flush();
                    }
                }
            }
            Event::StartIndentedParagraph => {
                let mark = self.out.len();
                self.newline();
                self.out.push_str(".IP\n");
                self.stack.push(Frame::IndentedParagraph { mark });
            }
            Event::EndIndentedParagraph => {
                if let Some(Frame::IndentedParagraph { mark }) = self.stack.pop() {
                    self.out.push('\n');
                    if !self.accepts_blocks() {
                        self.out.truncate(mark);
                    } else if self.stack.is_empty() {
                        self.flush();
                    }
                }
            }
            Event::StartHeading { level } => {
                self.stack.push(Frame::Heading {
                    level,
                    text: String::new(),
                });
            }
            Event::EndHeading => {
                if let Some(Frame::Heading { level, text }) = self.stack.pop()
                    && self.accepts_blocks()
                {
                    self.newline();
                    let macro_name = if level <= 2 { ".SH" } else { ".SS" };
                    self.out.push_str(macro_name);
                    self.out.push(' ');
                    self.out.push_str(&text.to_uppercase());
                    self.out.push('\n');
                    if self.stack.is_empty() {
                        self.flush();
                    }
                }
                // If the parent doesn't accept blocks, nothing was ever
                // written to `out` for this heading (its text lived only in
                // the frame), so there's nothing to truncate.
            }
            Event::CodeBlock { content } => {
                if self.accepts_blocks() {
                    self.newline();
                    self.out.push_str(".nf\n");
                    for line in content.lines() {
                        if line.starts_with('.') {
                            self.out.push_str("\\&");
                        }
                        self.out.push_str(line);
                        self.out.push('\n');
                    }
                    self.out.push_str(".fi\n");
                    self.maybe_flush();
                }
            }
            Event::ExampleBlock { content } => {
                if self.accepts_blocks() {
                    self.newline();
                    self.out.push_str(".EX\n");
                    for line in content.lines() {
                        self.out.push_str(line);
                        self.out.push('\n');
                    }
                    self.out.push_str(".EE\n");
                    self.maybe_flush();
                }
            }
            Event::HorizontalRule => {
                if self.accepts_blocks() {
                    self.newline();
                    self.out.push_str(".sp\n");
                    self.maybe_flush();
                }
            }
            Event::Comment { text } => {
                if self.accepts_blocks() {
                    self.newline();
                    self.out.push_str(".\\\" ");
                    self.out.push_str(&text);
                    self.out.push('\n');
                    self.maybe_flush();
                }
            }
            Event::StartList { ordered } => {
                let mark = self.out.len();
                self.newline();
                self.stack.push(Frame::List {
                    mark,
                    ordered,
                    item_index: 0,
                });
            }
            Event::EndList => {
                if let Some(Frame::List { mark, .. }) = self.stack.pop() {
                    if !self.accepts_blocks() {
                        self.out.truncate(mark);
                    } else if self.stack.is_empty() {
                        self.flush();
                    }
                }
            }
            Event::StartListItem => {
                let mark = self.out.len();
                if let Some(Frame::List {
                    ordered,
                    item_index,
                    ..
                }) = self.stack.last_mut()
                {
                    if *ordered {
                        *item_index += 1;
                        self.out.push_str(&format!(".IP {item_index}.\n"));
                    } else {
                        self.out.push_str(".IP \\(bu\n");
                    }
                }
                self.stack.push(Frame::ListItem { mark });
            }
            Event::EndListItem => {
                if let Some(Frame::ListItem { mark }) = self.stack.pop()
                    && !matches!(self.stack.last(), Some(Frame::List { .. }))
                {
                    self.out.truncate(mark);
                }
            }
            Event::StartDefinitionList => {
                self.stack.push(Frame::DefinitionList);
            }
            Event::EndDefinitionList => {
                if let Some(Frame::DefinitionList) = self.stack.pop()
                    && self.stack.is_empty()
                {
                    self.flush();
                }
            }
            Event::StartDefinitionTerm => {
                let mark = self.out.len();
                self.newline();
                self.out.push_str(".TP\n");
                self.stack.push(Frame::DefinitionTerm { mark });
            }
            Event::EndDefinitionTerm => {
                if let Some(Frame::DefinitionTerm { mark }) = self.stack.pop() {
                    self.out.push('\n');
                    if !matches!(self.stack.last(), Some(Frame::DefinitionList)) {
                        self.out.truncate(mark);
                    }
                }
            }
            Event::StartDefinitionDesc => {
                let mark = self.out.len();
                self.stack.push(Frame::DefinitionDesc { mark });
            }
            Event::EndDefinitionDesc => {
                if let Some(Frame::DefinitionDesc { mark }) = self.stack.pop()
                    && !matches!(self.stack.last(), Some(Frame::DefinitionList))
                {
                    self.out.truncate(mark);
                }
            }

            // ── Inline events ───────────────────────────────────────────
            Event::Text(cow) => {
                if let Some(Frame::Heading { text, .. }) = self.stack.last_mut() {
                    text.push_str(&cow);
                } else if self.accepts_inline() {
                    self.out.push_str(&escape_man(&cow));
                }
            }
            Event::Code(cow) => {
                if let Some(Frame::Heading { text, .. }) = self.stack.last_mut() {
                    text.push_str(&cow);
                } else if self.accepts_inline() {
                    self.out.push_str("\\f(CW");
                    self.out.push_str(&escape_man(&cow));
                    self.out.push_str("\\fR");
                }
            }
            Event::StartBold => {
                if !self.in_heading() {
                    self.open_inline_span("\\fB", "\\fR");
                }
            }
            Event::EndBold => {
                if !self.in_heading() {
                    self.close_inline_span();
                }
            }
            Event::StartItalic => {
                if !self.in_heading() {
                    self.open_inline_span("\\fI", "\\fR");
                }
            }
            Event::EndItalic => {
                if !self.in_heading() {
                    self.close_inline_span();
                }
            }
            Event::StartSuperscript => {
                if !self.in_heading() {
                    self.open_inline_span("^{", "}");
                }
            }
            Event::EndSuperscript => {
                if !self.in_heading() {
                    self.close_inline_span();
                }
            }
            Event::StartSubscript => {
                if !self.in_heading() {
                    self.open_inline_span("_{", "}");
                }
            }
            Event::EndSubscript => {
                if !self.in_heading() {
                    self.close_inline_span();
                }
            }
            Event::StartLink { url } => {
                if !self.in_heading() {
                    let mark = self.out.len();
                    self.stack.push(Frame::Link {
                        mark,
                        url: url.into_owned(),
                    });
                }
                // Inside a heading, extract_text() drops the link wrapper
                // entirely (url included) and only keeps the flattened
                // children text — so no frame is pushed; the url is simply
                // discarded and children flatten straight into the
                // enclosing Heading frame's text buffer.
            }
            Event::EndLink => {
                if self.in_heading() {
                    return;
                }
                if let Some(Frame::Link { mark, url }) = self.stack.pop() {
                    self.out.push_str(" (");
                    self.out.push_str(&escape_man(&url));
                    self.out.push(')');
                    if !self.accepts_inline() {
                        self.out.truncate(mark);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OwnedManEvent;
    use crate::test_alloc::{ALLOCS, CURRENT, PEAK};
    use std::borrow::Cow;
    use std::sync::atomic::Ordering;

    fn synthetic_source(n: usize) -> String {
        let mut s = String::from(".TH BIGDOC 1 \"2024-01-01\" \"Source\" \"Manual\"\n");
        for i in 0..n {
            s.push_str(&format!(
                ".SH SECTION {i}\n\
                 .PP\n\
                 Some plain text with \\fBbold\\fR and \\fIitalic\\fR markup, and a \
                 \\f(CWcode span\\fR.\n\
                 .IP \\(bu\n\
                 first point {i}\n\
                 .IP \\(bu\n\
                 second point {i}\n\
                 .TP\n\
                 term {i}\n\
                 description {i}\n"
            ));
        }
        s
    }

    /// Regression guard: an incremental writer must not reintroduce
    /// per-block subtree reconstruction. Allocation count for feeding N
    /// events through `Writer` must stay near-linear in N.
    #[test]
    fn test_writer_no_subtree_reconstruction_blowup() {
        fn run(n: usize) -> usize {
            let input = synthetic_source(n);
            let events: Vec<OwnedManEvent> = crate::events::events(&input).collect();

            let before = ALLOCS.load(Ordering::Relaxed);
            let mut out = Vec::new();
            {
                let mut w = Writer::new(&mut out);
                for e in events {
                    w.write_event(e);
                }
                w.finish();
            }
            let after = ALLOCS.load(Ordering::Relaxed);
            std::hint::black_box(&out);
            after - before
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
    /// the tree-based `parse()` + `build()` baseline, on a large synthetic
    /// document. `#[ignore]`d because it prints rather than asserts a
    /// threshold — run with `cargo test -p man-fmt --release \
    /// test_writer_peak_memory_and_throughput_report -- --ignored \
    /// --nocapture` to see the numbers.
    #[test]
    #[ignore]
    fn test_writer_peak_memory_and_throughput_report() {
        /// Discards written bytes instead of retaining them, so peak memory
        /// reflects the Writer's own internal state, not a `Vec<u8>` sink
        /// re-accumulating the whole document.
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

        let events: Vec<OwnedManEvent> = crate::events::events(&input).collect();
        let (doc, _diags) = crate::parse::parse(&input);

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

        let baseline = CURRENT.with(|c| c.get());
        PEAK.with(|p| p.set(baseline));
        let start = std::time::Instant::now();
        let built = crate::emit::build(std::hint::black_box(&doc));
        let builder_elapsed = start.elapsed();
        let builder_peak = PEAK.with(|p| p.get()).saturating_sub(baseline);
        std::hint::black_box(&built);

        eprintln!(
            "man-fmt streaming Writer vs parse()+build() builder, {} bytes input, 5000 \
             sections:\n\
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

    #[test]
    fn test_writer_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedManEvent::StartHeading { level: 2 });
        w.write_event(OwnedManEvent::Text(Cow::Owned("NAME".to_string())));
        w.write_event(OwnedManEvent::EndHeading);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains(".SH NAME"), "got: {s:?}");
    }

    #[test]
    fn test_writer_paragraph() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedManEvent::StartParagraph);
        w.write_event(OwnedManEvent::Text(Cow::Owned("Hello world".to_string())));
        w.write_event(OwnedManEvent::EndParagraph);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("Hello world"), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = ".SH NAME\ntest \\- a test program\n.SH SYNOPSIS\ntest [options]\n";
        let evts: Vec<_> = crate::events::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted_text = String::from_utf8(bytes).unwrap();
        // The emitted text should re-parse without panicking and contain
        // the key content.
        let (doc_emit, _) = crate::parse::parse(&emitted_text);
        assert!(
            !doc_emit.blocks.is_empty(),
            "writer roundtrip should produce blocks"
        );
        assert!(
            emitted_text.contains("NAME"),
            "emitted text should contain NAME"
        );
        assert!(
            emitted_text.contains("SYNOPSIS"),
            "emitted text should contain SYNOPSIS"
        );
    }

    /// The streaming `Writer`'s output on `events()`-fed input must be
    /// byte-identical to `build()`'s output on the same document — the
    /// primary regression guard for both defects fixed here: the `.TH`
    /// title/section/date/source/manual fields, and the buffer-then-emit
    /// architecture.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            ".TH TEST 1 \"2024-01-01\" \"Version 1.0\"\n.SH NAME\ntest\n",
            ".SH NAME\ntest \\- a test program\n.SH SYNOPSIS\ntest [options]\n",
            ".PP\nSome \\fBbold\\fR and \\fIitalic\\fR and \\f(CWcode\\fR text.\n",
            ".nf\nsome code\nmore code\n.fi\n",
            ".EX\nexample\n.EE\n",
            ".TP\nterm one\ndescription one\n.TP\nterm two\ndescription two\n",
            ".IP 1.\nfirst\n.IP 2.\nsecond\n",
            ".IP \\(bu\nbullet one\n.IP \\(bu\nbullet two\n",
            ".sp\n",
            ".\\\" a comment\n",
            ".IP\nan indented paragraph\n",
            ".SH NAME with \\fBbold\\fR and a link http://x/\n",
        ];
        for input in inputs {
            let (doc, _) = crate::parse::parse(input);
            let built = crate::emit::build(&doc);

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

    #[test]
    fn test_writer_th_metadata_via_events() {
        let input = ".TH TEST 1 \"2024-01-01\" \"Version 1.0\"\n";
        let mut w = Writer::new(Vec::<u8>::new());
        for e in crate::events::events(input) {
            w.write_event(e);
        }
        let s = String::from_utf8(w.finish()).unwrap();
        assert!(
            s.starts_with(".TH TEST 1 \"2024-01-01\" \"Version 1.0\" \"\"\n"),
            "got: {s:?}"
        );
    }
}
