//! Step 2: `events()` — a pull iterator borrowing from the input.
//!
//! This needs a GAT (`type Event<'a>`) because the event type borrows
//! from the `&[u8]` passed to `events()`, not from `&self`/`Self`. Plain
//! associated types can't express a lifetime that varies per call.
//!
//! **Result: compiles cleanly.** Stable GATs (since 1.65) handle this
//! case fine — it's the textbook GAT use case (lending-free borrow tied
//! to an external input lifetime, not to `&mut self`), unlike the
//! genuinely-lending case the design doc calls out for why `StreamingParser`
//! can't be `Iterator`.

use crate::step1_parse_emit::{FormatCrate, Opml, Zip};

pub trait FormatCrateEvents: FormatCrate {
    type Event<'a>
    where
        Self: 'a;
    type EventIter<'a>: Iterator<Item = Self::Event<'a>>
    where
        Self: 'a;

    fn events(input: &[u8]) -> Self::EventIter<'_>;
}

impl FormatCrateEvents for Opml {
    type Event<'a> = opml_fmt::Event<'a>;
    type EventIter<'a> = opml_fmt::EventIter<'a>;

    fn events(input: &[u8]) -> Self::EventIter<'_> {
        opml_fmt::events(input)
    }
}

impl FormatCrateEvents for Zip {
    // NOTE (real finding, not hand-waved): zip-fmt's `EventIter<'a>`
    // yields `OwnedEvent` (== `Event<'static>`), not `Event<'a>` — because
    // decompression necessarily allocates, there is no borrow from the
    // input to hand back. `type Event<'a>` here simply doesn't use `'a`,
    // which the GAT machinery permits, but it means the *design doc's*
    // general "Cow::Borrowed where the format allows zero-copy" framing
    // already has to special-case per-format, and a shared trait's GAT
    // must tolerate an impl that ignores its own lifetime parameter.
    type Event<'a> = zip_fmt::events::OwnedEvent;
    type EventIter<'a> = zip_fmt::EventIter<'a>;

    fn events(input: &[u8]) -> Self::EventIter<'_> {
        zip_fmt::events(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_events<F: FormatCrateEvents>(input: &[u8]) -> usize {
        F::events(input).count()
    }

    #[test]
    fn opml_generic_events() {
        let sample = br#"<opml version="2.0"><head><title>T</title></head><body><outline text="A"/></body></opml>"#;
        assert!(count_events::<Opml>(sample) > 0);
    }
}
