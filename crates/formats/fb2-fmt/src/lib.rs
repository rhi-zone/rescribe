//! FictionBook 2 (FB2) tokenizer, AST, and builder.
//!
//! A standalone crate with **no rescribe dependency**.
//! The `rescribe-read-fb2` and `rescribe-write-fb2` crates are thin adapters.
//!
//! # API
//!
//! ```text
//! // AST round-trip
//! pub fn parse(input: &[u8]) -> (FictionBook, Vec<Diagnostic>);
//! pub fn parse_str(input: &str) -> (FictionBook, Vec<Diagnostic>);
//! pub fn emit(fb: &FictionBook) -> Vec<u8>;
//!
//! // Pull iterator — true incremental XML parsing
//! pub fn events(input: &[u8]) -> EventIter;
//!
//! // Chunk-driven streaming parser
//! // StreamingParser<H: Handler> + Handler trait
//! ```

pub mod ast;
mod emit;
mod events;
mod parse;
pub mod writer;

pub use ast::*;
pub use emit::emit;
pub use events::{Event, EventIter, Handler, StreamingParser, events};
pub use parse::{parse, parse_str};

#[cfg(test)]
mod whitespace_tests {
    use crate::{emit, parse};
    use crate::ast::*;

    fn roundtrip_inline(el: InlineElement) -> InlineElement {
        let mut fb = FictionBook::default();
        let section = Section {
            content: vec![SectionContent::Para(vec![el])],
            ..Default::default()
        };
        fb.bodies.push(Body { section: vec![section], ..Default::default() });
        let bytes = emit(&fb);
        let (fb2, _) = parse(&bytes);
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
