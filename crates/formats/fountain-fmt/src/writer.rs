//! Streaming Fountain writer — converts a stream of events to Fountain text.
//!
//! Classification of every construct `events()` produces:
//!
//! - **Write-straight-through** (no buffering beyond the current byte, once
//!   any prefix is known): `SceneHeading`, `Action`, `Dialogue`,
//!   `Parenthetical`, `Centered`, `Lyric`, `Note`, `Synopsis`, `Section`,
//!   `PageBreak`, `Boneyard`. None of these transform their text (no case
//!   change, no conditional prefix based on the *whole* string), so their
//!   fixed prefix is written at `Start*` and each `Text` event is appended
//!   immediately.
//! - **`StartDialogueBlock`/`EndDialogueBlock`**: pure grouping wrapper with
//!   zero output effect — `ast::Block` has no `DialogueBlock` variant;
//!   `emit::emit_block` never sees one. No frame is pushed for it.
//! - **Genuinely deferred, O(one block's text)**: `Character` (needs the
//!   full name before uppercasing + appending the dual-dialogue `" ^"`
//!   suffix) and `Transition` (needs the full text before uppercasing +
//!   deciding whether to force a `>` prefix based on whether it already
//!   ends in `"TO:"`). Both accumulate into a small `String` on their frame,
//!   bounded by that one block's text — not the document — exactly the same
//!   category as rst-fmt's heading-underline deferral.
//! - **Genuinely deferred, O(nesting depth) persistent state, not a
//!   buffer**: `ensure_blank_line` (mirroring `emit::BuildContext::
//!   ensure_blank_line`) inspects the *tail* of everything written so far to
//!   decide whether a blank line is needed before the next block. Because
//!   `out` is cleared after each flush, that tail can't be read back out of
//!   `out` itself — so a tiny `Trailing` enum (0/1/2+ trailing newlines,
//!   updated on every push) tracks it persistently instead.
//! - **Genuinely deferred, O(metadata field count)**: title-page
//!   `Metadata { key, value }` events all arrive contiguously right after
//!   `StartDocument` (see `events.rs`'s `Phase::Metadata`), but
//!   `emit::emit_title_page` reorders them into a fixed field order before
//!   any extras — so they're buffered in a small map (bounded by the
//!   number of metadata fields, not document size) and flushed in that
//!   canonical order the moment the first non-metadata event arrives.
//!
//! # Example
//! ```no_run
//! use fountain_fmt::writer::Writer;
//! use fountain_fmt::OwnedEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! w.write_event(OwnedEvent::StartSceneHeading);
//! w.write_event(OwnedEvent::Text("INT. OFFICE - DAY".to_string().into()));
//! w.write_event(OwnedEvent::EndSceneHeading);
//! let bytes = w.finish();
//! ```

use crate::events::OwnedEvent;
use std::collections::BTreeMap;
use std::io::Write;

/// Streaming Fountain writer.
///
/// Feed events with [`write_event`](Writer::write_event); each top-level
/// construct is emitted to the sink as soon as it closes. Call
/// [`finish`](Writer::finish) to flush any remainder and recover the sink.
pub struct Writer<W: Write> {
    sink: W,
    /// Shared output buffer. Cleared (capacity retained) after each
    /// top-level block is flushed.
    out: String,
    /// Frame stack for the block currently being assembled. Empty at top
    /// level — closing a construct with an empty stack flushes.
    stack: Vec<Frame>,
    /// Persists across `out` being cleared, so `ensure_blank_line` still
    /// knows what the last flushed bytes ended with. See the module doc.
    trailing: Trailing,
    /// Whether anything has been written yet (`ensure_blank_line` is a
    /// no-op before the very first block, unlike `Trailing::Zero`, which
    /// means "wrote something that doesn't end in a newline").
    any_output: bool,
    /// Buffered title-page metadata (see the module doc's O(metadata field
    /// count) note). `None` once flushed.
    metadata: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Trailing {
    Zero,
    One,
    TwoPlus,
}

enum Frame {
    SceneHeading,
    Action,
    Character { name: String, dual: bool },
    Dialogue,
    Parenthetical,
    Transition { text: String },
    Centered,
    Lyric,
    Note,
    Synopsis,
    Section,
    Boneyard,
}

const DEFAULT_OUT_CAPACITY: usize = 4096;
const TITLE_PAGE_FIELD_ORDER: [&str; 9] = [
    "title",
    "credit",
    "author",
    "authors",
    "source",
    "draft_date",
    "contact",
    "copyright",
    "notes",
];

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
            trailing: Trailing::Zero,
            any_output: false,
            metadata: Some(BTreeMap::new()),
        }
    }

    /// Feed one event to the writer. Writes bytes to the sink immediately
    /// whenever this event completes a top-level construct.
    pub fn write_event(&mut self, event: OwnedEvent) {
        // Metadata events accumulate; `StartDocument` always precedes them
        // (see `events.rs`'s `Phase::Metadata` starting only after
        // `StartDocument` is emitted), so it must NOT trigger the flush —
        // doing so would take (and permanently drop) the still-empty
        // metadata map before any real `Metadata` events arrive. Every
        // other event first flushes any pending title page (a no-op once
        // already flushed), then proceeds.
        match &event {
            OwnedEvent::Metadata { key, value } => {
                if let Some(meta) = &mut self.metadata {
                    meta.insert(key.clone().into_owned(), value.clone().into_owned());
                }
                return;
            }
            OwnedEvent::StartDocument => {}
            _ => self.flush_metadata_if_pending(),
        }
        self.process(event);
        self.maybe_flush();
    }

    /// Flush any remaining buffered bytes and recover the sink.
    pub fn finish(mut self) -> W {
        self.flush_metadata_if_pending();
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

    /// Append to the shared buffer, updating the persistent trailing-newline
    /// state so it survives `out` being cleared by a flush.
    fn push(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.out.push_str(s);
        self.any_output = true;
        let bytes = s.as_bytes();
        let mut trailing_in_s = 0usize;
        for &b in bytes.iter().rev() {
            if b == b'\n' && trailing_in_s < 2 {
                trailing_in_s += 1;
            } else {
                break;
            }
        }
        self.trailing = if trailing_in_s == bytes.len() {
            // `s` was entirely newlines (1 or 2 of them): they extend
            // whatever trailing run preceded `s`.
            match (self.trailing, trailing_in_s) {
                (Trailing::Zero, 1) => Trailing::One,
                (_, _) if trailing_in_s >= 2 => Trailing::TwoPlus,
                (Trailing::One, 1) | (Trailing::TwoPlus, 1) => Trailing::TwoPlus,
                _ => Trailing::One,
            }
        } else {
            match trailing_in_s {
                0 => Trailing::Zero,
                1 => Trailing::One,
                _ => Trailing::TwoPlus,
            }
        };
    }

    /// Mirrors `emit::BuildContext::ensure_blank_line`: guarantee exactly
    /// one blank line before the next block, using persistent state instead
    /// of re-inspecting `out` (which may have already been flushed away).
    fn ensure_blank_line(&mut self) {
        if !self.any_output {
            return;
        }
        match self.trailing {
            Trailing::TwoPlus => {}
            Trailing::One => self.push("\n"),
            Trailing::Zero => self.push("\n\n"),
        }
    }

    fn flush_metadata_if_pending(&mut self) {
        let Some(metadata) = self.metadata.take() else {
            return;
        };
        if metadata.is_empty() {
            return;
        }
        let mut has_output = false;
        let write_field = |this: &mut Self, key: &str, value: &str| {
            let display_key = key
                .split('_')
                .map(|s| {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().chain(c).collect(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            this.push(&display_key);
            this.push(": ");
            this.push(value);
            this.push("\n");
        };
        for field in TITLE_PAGE_FIELD_ORDER {
            if let Some(value) = metadata.get(field) {
                write_field(self, field, value);
                has_output = true;
            }
        }
        for (key, value) in metadata.iter() {
            if !TITLE_PAGE_FIELD_ORDER.contains(&key.as_str()) {
                write_field(self, key, value);
                has_output = true;
            }
        }
        if has_output {
            self.push("\n");
        }
    }

    fn process(&mut self, event: OwnedEvent) {
        match event {
            OwnedEvent::StartDocument | OwnedEvent::EndDocument => {}
            OwnedEvent::Metadata { .. } => unreachable!("handled in write_event"),

            OwnedEvent::StartDialogueBlock | OwnedEvent::EndDialogueBlock => {}

            OwnedEvent::StartSceneHeading => {
                self.ensure_blank_line();
                self.stack.push(Frame::SceneHeading);
            }
            OwnedEvent::EndSceneHeading => {
                if matches!(self.stack.last(), Some(Frame::SceneHeading)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }

            OwnedEvent::StartAction => {
                self.ensure_blank_line();
                self.stack.push(Frame::Action);
            }
            OwnedEvent::EndAction => {
                if matches!(self.stack.last(), Some(Frame::Action)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }

            OwnedEvent::StartCharacter { dual } => {
                self.ensure_blank_line();
                self.stack.push(Frame::Character {
                    name: String::new(),
                    dual,
                });
            }
            OwnedEvent::EndCharacter => {
                if let Some(Frame::Character { name, dual }) = self.stack.pop() {
                    self.push(&name.to_uppercase());
                    if dual {
                        self.push(" ^");
                    }
                    self.push("\n");
                }
            }

            OwnedEvent::StartDialogue => self.stack.push(Frame::Dialogue),
            OwnedEvent::EndDialogue => {
                if matches!(self.stack.last(), Some(Frame::Dialogue)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }

            OwnedEvent::StartParenthetical => self.stack.push(Frame::Parenthetical),
            OwnedEvent::EndParenthetical => {
                if matches!(self.stack.last(), Some(Frame::Parenthetical)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }

            OwnedEvent::StartTransition => {
                self.ensure_blank_line();
                self.stack.push(Frame::Transition {
                    text: String::new(),
                });
            }
            OwnedEvent::EndTransition => {
                if let Some(Frame::Transition { text }) = self.stack.pop() {
                    let upper = text.to_uppercase();
                    if !upper.ends_with("TO:") {
                        self.push(">");
                    }
                    self.push(&upper);
                    self.push("\n");
                }
            }

            OwnedEvent::StartCentered => {
                self.ensure_blank_line();
                self.push(">");
                self.stack.push(Frame::Centered);
            }
            OwnedEvent::EndCentered => {
                if matches!(self.stack.last(), Some(Frame::Centered)) {
                    self.stack.pop();
                    self.push("<\n");
                }
            }

            OwnedEvent::StartLyric => {
                self.push("~");
                self.stack.push(Frame::Lyric);
            }
            OwnedEvent::EndLyric => {
                if matches!(self.stack.last(), Some(Frame::Lyric)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }

            OwnedEvent::StartNote => {
                self.push("[[");
                self.stack.push(Frame::Note);
            }
            OwnedEvent::EndNote => {
                if matches!(self.stack.last(), Some(Frame::Note)) {
                    self.stack.pop();
                    self.push("]]\n");
                }
            }

            OwnedEvent::StartSynopsis => {
                self.push("= ");
                self.stack.push(Frame::Synopsis);
            }
            OwnedEvent::EndSynopsis => {
                if matches!(self.stack.last(), Some(Frame::Synopsis)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }

            OwnedEvent::StartSection { level } => {
                self.ensure_blank_line();
                self.push(&"#".repeat(level));
                self.push(" ");
                self.stack.push(Frame::Section);
            }
            OwnedEvent::EndSection => {
                if matches!(self.stack.last(), Some(Frame::Section)) {
                    self.stack.pop();
                    self.push("\n");
                }
            }

            OwnedEvent::PageBreak => {
                self.ensure_blank_line();
                self.push("===\n");
            }

            OwnedEvent::StartBoneyard => {
                self.ensure_blank_line();
                self.push("/* ");
                self.stack.push(Frame::Boneyard);
            }
            OwnedEvent::EndBoneyard => {
                if matches!(self.stack.last(), Some(Frame::Boneyard)) {
                    self.stack.pop();
                    self.push(" */\n");
                }
            }

            OwnedEvent::Text(cow) => match self.stack.last_mut() {
                Some(Frame::Character { name, .. }) => name.push_str(&cow),
                Some(Frame::Transition { text }) => text.push_str(&cow),
                Some(
                    Frame::SceneHeading
                    | Frame::Action
                    | Frame::Dialogue
                    | Frame::Parenthetical
                    | Frame::Centered
                    | Frame::Lyric
                    | Frame::Note
                    | Frame::Synopsis
                    | Frame::Section
                    | Frame::Boneyard,
                ) => {
                    let s = cow.into_owned();
                    self.push(&s);
                }
                None => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_scene_heading() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartDocument);
        w.write_event(OwnedEvent::StartSceneHeading);
        w.write_event(OwnedEvent::Text("INT. OFFICE - DAY".to_string().into()));
        w.write_event(OwnedEvent::EndSceneHeading);
        w.write_event(OwnedEvent::EndDocument);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("INT. OFFICE - DAY"), "got: {s:?}");
    }

    #[test]
    fn test_writer_action() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::StartDocument);
        w.write_event(OwnedEvent::StartAction);
        w.write_event(OwnedEvent::Text("John enters.".to_string().into()));
        w.write_event(OwnedEvent::EndAction);
        w.write_event(OwnedEvent::EndDocument);
        let bytes = w.finish();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("John enters."), "got: {s:?}");
    }

    #[test]
    fn test_writer_roundtrip_via_events() {
        let input = "INT. OFFICE - DAY\n\nJohn sits down.\n\nCUT TO:\n";
        let evts: Vec<_> = crate::events::events(input).collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evts {
            w.write_event(e);
        }
        let bytes = w.finish();
        let emitted = String::from_utf8(bytes).unwrap();
        let (doc_orig, _) = crate::parse::parse(input);
        let (doc_emit, _) = crate::parse::parse(&emitted);
        assert_eq!(
            doc_orig.blocks.len(),
            doc_emit.blocks.len(),
            "writer roundtrip block count mismatch"
        );
    }

    /// The streaming `Writer` must produce byte-identical output to the
    /// tree-based `emit::emit` for the same document — including dual
    /// dialogue and a title page, the two constructs most likely to need
    /// real deferral.
    #[test]
    fn test_writer_byte_identical_to_builder() {
        let inputs = [
            "INT. OFFICE - DAY\n\nJohn sits down.\n\nCUT TO:\n",
            "Title: My Screenplay\nAuthor: Jane Doe\nDraft date: 2024-01-01\n\nINT. ROOM - DAY\n\nSomething happens.\n",
            "INT. ROOM - DAY\n\nJOHN\nHello there.\n\nJANE ^\nHi John.\n",
            "> CENTERED TEXT <\n",
            "~Singing a song\n~another line\n",
            "[[a note]]\n",
            "= a synopsis line\n",
            "# Act One\n\n## Scene One\n\nSome action.\n",
            "===\n",
            "/* a boneyard comment */\n",
            "FADE OUT.\n",
            "INT. HALL - DAY\n\nJOHN\n(whispering)\nQuiet.\n",
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

    /// Regression guard against reintroducing per-event `Block` subtree
    /// reconstruction.
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

        fn events_for(n: usize) -> Vec<OwnedEvent> {
            let mut evs = Vec::new();
            for i in 0..n {
                evs.push(OwnedEvent::StartSceneHeading);
                evs.push(OwnedEvent::Text(format!("INT. ROOM {i} - DAY").into()));
                evs.push(OwnedEvent::EndSceneHeading);
                evs.push(OwnedEvent::StartAction);
                evs.push(OwnedEvent::Text(format!("Something happens {i}.").into()));
                evs.push(OwnedEvent::EndAction);
                evs.push(OwnedEvent::StartDialogueBlock);
                evs.push(OwnedEvent::StartCharacter { dual: false });
                evs.push(OwnedEvent::Text("JOHN".to_string().into()));
                evs.push(OwnedEvent::EndCharacter);
                evs.push(OwnedEvent::StartDialogue);
                evs.push(OwnedEvent::Text(format!("Line number {i}.").into()));
                evs.push(OwnedEvent::EndDialogue);
                evs.push(OwnedEvent::EndDialogueBlock);
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
                w.write_event(OwnedEvent::StartDocument);
                for e in evs {
                    w.write_event(e);
                }
                w.write_event(OwnedEvent::EndDocument);
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

        // Peak memory should also stay roughly constant across document
        // sizes (a ratio, not an absolute byte ceiling, for the same reason
        // given in `pod-fmt`/`haddock-fmt`'s equivalent tests: this test
        // binary's threads share one process-wide `#[global_allocator]`, so
        // an absolute threshold isn't robust to unrelated concurrent tests'
        // allocation noise).
        fn run_peak(n: usize) -> usize {
            PEAK_BYTES.store(0, Ordering::SeqCst);
            let before = CURRENT_BYTES.load(Ordering::SeqCst);
            let evs = events_for(n);
            let mut out = Vec::new();
            {
                let mut w = Writer::new(&mut out);
                w.write_event(OwnedEvent::StartDocument);
                for e in evs {
                    w.write_event(e);
                }
                w.write_event(OwnedEvent::EndDocument);
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
