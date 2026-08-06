//! FictionBook 2 (FB2) tokenizer, AST, and builder.
//!
//! A standalone crate with **no rescribe dependency by default**. The
//! optional `rescribe` feature (default off) adds `crate::rescribe::{parse,
//! emit}`, a thin adapter that translates `fb2_fmt::FictionBook` to and
//! from rescribe's `Document` IR.
//!
//! # API layers
//!
//! `FictionBook` implements all five shared `rescribe-format-api` traits —
//! its native reader/writer functions already took `&[u8]`/returned
//! `Vec<u8>`, so the trait impls are thin, direct wrappers with no
//! signature-mismatch work:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! // AST reader
//! let (fb, diags): (FictionBook, Vec<Diagnostic>) = FictionBook::parse(input);
//!
//! // Streaming reader — true incremental XML parsing
//! let it: EventIter = FictionBook::events(input);
//!
//! // Batch reader — chunk-driven
//! let mut p = FictionBook::streaming_parser(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer
//! let bytes: Vec<u8> = fb.emit();
//!
//! // Streaming writer
//! let mut w = FictionBook::writer(sink);
//! w.write_event(event); // repeat
//! w.finish();
//! ```
//!
//! `parse_str` (takes `&str`, skips the UTF-8 check `Parse::parse` implies)
//! remains a separate, documented entry point alongside the trait methods,
//! matching `commonmark-fmt`'s precedent for a materially different
//! contract.

pub mod ast;
mod emit;
mod events;
mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
pub mod writer;

pub use ast::*;
pub use events::{Event, EventIter, StreamingParser};
pub use parse::parse_str;
pub use rescribe_format_api::{Emit, Events, Handler, Parse, StreamingParse, StreamingWrite};
pub use writer::Writer;

// ── Trait implementations ───────────────────────────────────────────────────
//
// `FictionBook` implements the shared API-mode traits directly — no
// parallel free functions (`fb2_fmt::parse(..)`, `fb2_fmt::emit(..)`, ...)
// exist alongside them. `parse_str` stays public (materially different
// contract: str input).

impl Parse for FictionBook {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        parse::parse(input)
    }
}

impl Emit for FictionBook {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self)
    }
}

impl Events for FictionBook {
    type Event<'a> = Event;
    type EventIter<'a> = EventIter<'a>;

    fn events(input: &[u8]) -> EventIter<'_> {
        events::events(input)
    }
}

impl StreamingParse for FictionBook {
    type Event = Event;
    type Parser<H: Handler<Event>> = StreamingParser<H>;

    fn streaming_parser<H: Handler<Event>>(handler: H) -> StreamingParser<H> {
        StreamingParser::new(handler)
    }
}

impl StreamingWrite for FictionBook {
    type Writer<W: std::io::Write> = Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> Writer<W> {
        Writer::new(sink)
    }
}

#[cfg(test)]
mod whitespace_tests {
    use crate::ast::*;
    use crate::{Emit as _, Parse as _};

    fn roundtrip_inline(el: InlineElement) -> InlineElement {
        let mut fb = FictionBook::default();
        let section = Section {
            content: vec![SectionContent::Para(vec![el])],
            ..Default::default()
        };
        fb.bodies.push(Body {
            section: vec![section],
            ..Default::default()
        });
        let bytes = fb.emit();
        let (fb2, _) = FictionBook::parse(&bytes);
        match &fb2.bodies[0].section[0].content[0] {
            SectionContent::Para(inlines) => inlines[0].clone(),
            _ => panic!("not para"),
        }
    }

    #[test]
    fn code_preserves_leading_space() {
        // Regression: <code> content was trimmed during parse, dropping leading spaces.
        let result = roundtrip_inline(InlineElement::Code(" hello".to_string()));
        assert_eq!(result, InlineElement::Code(" hello".to_string()));
    }

    #[test]
    fn code_preserves_trailing_space() {
        let result = roundtrip_inline(InlineElement::Code("hello ".to_string()));
        assert_eq!(result, InlineElement::Code("hello ".to_string()));
    }
}
