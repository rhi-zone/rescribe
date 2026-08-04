//! MultiMarkdown parser and emitter, built on `commonmark-fmt`.
//!
//! A standalone crate with **no rescribe dependency** — usable as a general
//! Rust MultiMarkdown library. `rescribe-read-multimarkdown` and
//! `rescribe-write-multimarkdown` are thin adapter layers on top.
//!
//! # Design: extend, don't reimplement
//!
//! MultiMarkdown is CommonMark/GFM plus footnotes, definition lists, math,
//! and a handful of MMD-only constructs layered on top — it is not an
//! independent grammar. Per repository CLAUDE.md's decision for this
//! vertical, this crate depends on `commonmark-fmt` (its `tables`,
//! `task-lists`, `strikethrough`, `footnotes`, `definition-lists`, and
//! `math` features) for every construct CommonMark/GFM already defines, and
//! implements only the genuinely MMD-unique remainder:
//!
//! - **Metadata blocks** (`crate::metadata`) — `Key: value` lines at the top
//!   of the document, with or without `---`/`...` delimiters. Distinct
//!   grammar from commonmark-fmt's `frontmatter` feature (which requires
//!   delimiters and doesn't parse the interior) — implemented directly here.
//! - **Citations** (`[locator][#refname]`, `[#refname]: definition`) and
//!   **cross-references** (`[Anchor]`, `[Header Text][]`) — spelled with
//!   ordinary link-reference bracket syntax that CommonMark's own parser
//!   already leaves as literal text when unresolved (see `crate::citation`
//!   for exactly how that fallback text is recognized).
//!
//! [`ast::MmdBlock`]/[`ast::MmdInline`] mirror `commonmark_fmt::{Block,
//! Inline}` variant-for-variant (commonmark-fmt's AST is a closed enum with
//! no extension point, so this is the only way to let a citation or
//! cross-reference appear anywhere in the tree that plain text can) plus the
//! MMD-unique variants above. See `crate::transform` for the conversion
//! both directions, and `ast.rs`'s module doc for the full rationale.
//!
//! # API layers
//!
//! ```text
//! // AST reader
//! pub fn parse(input: &[u8]) -> (MmdDoc, Vec<Diagnostic>);
//!
//! // Streaming reader — true incremental iterator, see events.rs's module doc
//! pub fn events(input: &[u8]) -> Option<EventIter<'_>>;
//!
//! // Batch reader — chunk-driven
//! let mut p = StreamingParser::new(|ev| ...);
//! p.feed(chunk); // repeat
//! p.finish();
//!
//! // Builder writer — emit from AST
//! pub fn emit(doc: &MmdDoc) -> Vec<u8>;
//!
//! // Streaming writer — emit from events
//! let mut w = Writer::new(sink);
//! w.write_event(event); // repeat
//! w.finish(); // flushes to sink
//! ```
//!
//! # Limitations
//!
//! **`StreamingParser` buffers all input before parsing.** This is not a
//! limitation multimarkdown-fmt introduces — it inherits commonmark-fmt's
//! own documented pulldown-cmark exemption (pulldown-cmark requires the
//! full input as `&str`) for the CommonMark body beneath MMD's extensions.
//! See `crate::batch`'s module doc for the full explanation. [`events`] is
//! unaffected: it is a true streaming iterator with O(1) event lookahead,
//! not built on top of `parse`. See `crate::events`'s module doc.
//!
//! **The event-stream API loses metadata delimiter style** (bare vs.
//! `---`-delimited) on a round trip; the AST path preserves it exactly. See
//! `crate::writer`'s module doc.
//!
//! **Citation/cross-reference recognition operates within a single
//! contiguous text run** — a locator or label containing nested Markdown
//! markup is left as plain inline content rather than upgraded to a
//! structured node. See `crate::citation`'s module doc.
//!
//! Superseding pulldown-cmark (77M+ weekly downloads) is a non-goal, same as
//! commonmark-fmt.

pub mod ast;
pub mod batch;
pub mod citation;
pub mod emit;
pub mod events;
pub mod metadata;
pub mod parse;
pub mod transform;
pub mod writer;

pub use ast::{
    ColumnAlignment, Diagnostic, LinkDef, ListKind, MetadataEntry, MetadataStyle, MmdBlock,
    MmdDefinitionListItem, MmdDoc, MmdInline, MmdListItem, MmdTableCell, MmdTableRow,
    OrderedMarker, Span,
};
pub use batch::{Handler, StreamingParser};
pub use emit::emit;
pub use events::{Event, EventIter, OwnedEvent, events, events_str};
pub use parse::parse;
pub use writer::Writer;

#[cfg(test)]
mod smoke {
    use super::*;

    const SAMPLE: &str = "Title: Sample Document\nAuthor: Jane Doe\n\n\
        # Overview [MultiMarkdownOverview] ##\n\n\
        See[p. 23][#Doe:2006] for details, and [Overview][] above.\n\n\
        [#Doe:2006]: John Doe. *A Great Book*. 2006.\n";

    #[test]
    fn smoke_parse() {
        let (doc, diags) = parse(SAMPLE.as_bytes());
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.metadata_style, MetadataStyle::Bare);
        assert_eq!(doc.metadata.len(), 2);
        assert_eq!(doc.blocks.len(), 3);
        assert!(matches!(
            &doc.blocks[0],
            MmdBlock::Heading { anchor: Some(a), .. } if a == "MultiMarkdownOverview"
        ));
        assert!(matches!(
            &doc.blocks[2],
            MmdBlock::CitationDefinition { .. }
        ));
    }

    #[test]
    fn smoke_roundtrip() {
        let (doc1, diags1) = parse(SAMPLE.as_bytes());
        assert!(diags1.is_empty());
        let emitted = emit(&doc1);
        let (doc2, diags2) = parse(&emitted);
        assert!(diags2.is_empty(), "diagnostics: {diags2:?}");
        assert_eq!(doc1.strip_spans(), doc2.strip_spans(), "roundtrip mismatch");
    }

    #[test]
    fn smoke_events() {
        let evs: Vec<_> = events(SAMPLE.as_bytes()).unwrap().collect();
        assert!(!evs.is_empty());
        assert!(evs.iter().any(|e| matches!(e, Event::Metadata { .. })));
        assert!(evs.iter().any(|e| matches!(e, Event::Citation { .. })));
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::StartCitationDefinition { .. }))
        );
    }

    #[test]
    fn smoke_batch_parser() {
        let mut evs = Vec::new();
        let mut p = StreamingParser::new(|ev| evs.push(ev));
        let mid = SAMPLE.len() / 2;
        p.feed(&SAMPLE.as_bytes()[..mid]);
        p.feed(&SAMPLE.as_bytes()[mid..]);
        p.finish();
        assert!(!evs.is_empty());
        assert!(evs.iter().any(|e| matches!(e, OwnedEvent::Metadata { .. })));
    }

    #[test]
    fn smoke_writer() {
        let (doc, _diags) = parse(SAMPLE.as_bytes());
        let evs: Vec<_> = events(SAMPLE.as_bytes())
            .unwrap()
            .map(|e| e.into_owned())
            .collect();
        let mut w = Writer::new(Vec::<u8>::new());
        for ev in evs {
            w.write_event(ev);
        }
        let bytes = w.finish().unwrap();
        // Streaming writer content-equivalent to the builder path (modulo
        // metadata delimiter style, which events() doesn't carry — see
        // crate::writer's module doc).
        let (doc2, diags2) = parse(&bytes);
        assert!(diags2.is_empty(), "diagnostics: {diags2:?}");
        assert_eq!(doc.strip_spans().blocks, doc2.strip_spans().blocks);
        assert_eq!(doc.metadata, doc2.metadata);
    }
}
