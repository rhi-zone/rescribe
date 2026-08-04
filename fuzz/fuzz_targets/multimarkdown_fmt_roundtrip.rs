#![no_main]

//! multimarkdown-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary `MmdDoc` from fuzz data, emits it to MultiMarkdown
//! bytes, parses back, and asserts structural equality (after
//! `strip_spans()`).
//!
//! Direction: arbitrary_mmd_ast -> emit -> parse -> assert equality
//!
//! This is the definitive roundtrip test per CLAUDE.md: starts from the
//! format crate's own `Ast` type (not the IR).
//!
//! The generator is deliberately conservative, mirroring typst-fmt's own
//! roundtrip fuzz target's rationale (see that file's module doc): text
//! content is restricted to lowercase ASCII letters and single spaces —
//! never `[`, `]`, `:`, `#`, `^`, `\n`, or CommonMark's own inline-markup
//! delimiters (`*`, `_`, `` ` ``, `<`, `\`, `~`) — so that:
//! - generated plain text can never accidentally look like a metadata
//!   `Key: value` line (which needs a literal `:`) or a citation/
//!   cross-reference bracket pattern (which needs `[`/`]`) on reparse;
//! - two adjacent generated inlines never get merged or re-split by
//!   CommonMark's own tokenizer in a way that would break the structural
//!   equality check for reasons unrelated to the property under test.
//! Every generated `Citation`/`CrossReference`/text-bearing node is
//! separated from its neighbors by a literal space when adjacency could
//! otherwise be ambiguous, adjacent generated `Text` nodes are merged at
//! generation time (CommonMark's tokenizer always merges them back into one
//! on reparse), and a heading's own trailing shortcut-form `CrossReference`
//! is forced to collapsed form (`crate::transform::extract_heading_anchor`
//! always reads a bare trailing one back as an explicit `anchor` instead —
//! see `Gen::block`'s heading arm). Checked by running this generator's
//! logic standalone against 300k pseudo-random byte buffers with zero
//! mismatches before being committed here (`cargo fuzz` itself is not
//! available in this development environment).

use libfuzzer_sys::fuzz_target;
use multimarkdown_fmt::{MetadataEntry, MetadataStyle, MmdBlock, MmdDoc, MmdInline, Span};

/// Push a space onto `v`, merging into a trailing `Text` node if there is
/// one (see `Gen::inlines`'s doc comment on why merging is required).
fn push_text(v: &mut Vec<MmdInline>, s: &str) {
    if let Some(MmdInline::Text { content, .. }) = v.last_mut() {
        content.push_str(s);
    } else {
        v.push(MmdInline::Text {
            content: s.to_string(),
            span: Span::NONE,
        });
    }
}

struct Gen<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Gen<'a> {
    fn new(data: &'a [u8]) -> Self {
        Gen { data, pos: 0 }
    }

    fn byte(&mut self) -> u8 {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            b
        } else {
            0
        }
    }

    fn maybe(&mut self) -> bool {
        self.byte() % 2 == 0
    }

    /// Lowercase ASCII letters only, never empty. Safe against every
    /// ambiguity class documented above.
    fn word(&mut self, n: usize) -> String {
        let s: String = (0..n)
            .map(|_| ((self.byte() % 26) + b'a') as char)
            .collect();
        if s.is_empty() { "x".to_string() } else { s }
    }

    /// A short phrase of 1-3 words separated by single spaces — still no
    /// colons/brackets, but exercises the metadata continuation and
    /// citation-locator "contains a space" cases.
    fn phrase(&mut self, words: usize) -> String {
        (0..words.max(1))
            .map(|_| self.word(4))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn inline(&mut self, depth: u8) -> MmdInline {
        if depth >= 2 {
            return MmdInline::Text {
                content: self.word(5),
                span: Span::NONE,
            };
        }
        match self.byte() % 5 {
            0 => MmdInline::Emphasis {
                inlines: vec![MmdInline::Text {
                    content: self.word(5),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            },
            1 => MmdInline::Strong {
                inlines: vec![MmdInline::Text {
                    content: self.word(5),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            },
            2 => MmdInline::Citation {
                locator: self.maybe().then(|| self.phrase(2)),
                label: self.word(6),
                span: Span::NONE,
            },
            3 => MmdInline::CrossReference {
                target: self.phrase(2),
                collapsed: self.maybe(),
                span: Span::NONE,
            },
            _ => MmdInline::Text {
                content: self.word(5),
                span: Span::NONE,
            },
        }
    }

    /// A sequence of inlines, always separating any two non-Text nodes (or
    /// two Text nodes, which CommonMark would otherwise merge on reparse)
    /// with a literal space Text, so adjacency never introduces ambiguity
    /// unrelated to the property under test.
    fn inlines(&mut self, depth: u8) -> Vec<MmdInline> {
        let count = (self.byte() % 3) + 1;
        let mut v: Vec<MmdInline> = Vec::new();
        for i in 0..count {
            if i > 0 {
                push_text(&mut v, " ");
            }
            let item = self.inline(depth);
            match (v.last_mut(), &item) {
                // CommonMark's tokenizer merges any two adjacent plain-Text
                // runs into a single Text node on reparse (there is no way
                // to preserve a split point between them) — merge here too,
                // so this generator produces only trees `parse(emit(_))`
                // can actually reproduce.
                (
                    Some(MmdInline::Text { content: prev, .. }),
                    MmdInline::Text { content: cur, .. },
                ) => {
                    prev.push_str(cur);
                }
                _ => v.push(item),
            }
        }
        v
    }

    fn block(&mut self) -> MmdBlock {
        match self.byte() % 4 {
            0 => {
                let mut inlines = self.inlines(0);
                // A heading's *own* trailing shortcut-form CrossReference
                // (`[target]`, not `[target][]`) is indistinguishable from
                // an explicit `anchor` on reparse — `crate::transform`'s
                // `extract_heading_anchor` always reads it back as one (see
                // that function's doc comment). Force it to collapsed form
                // so it round-trips as the plain cross-reference it was
                // generated to be, regardless of whether `anchor` below is
                // also `Some`.
                if let Some(MmdInline::CrossReference {
                    collapsed: collapsed @ false,
                    ..
                }) = inlines.last_mut()
                {
                    *collapsed = true;
                }
                MmdBlock::Heading {
                    level: (self.byte() % 6) + 1,
                    inlines,
                    anchor: self.maybe().then(|| self.word(6)),
                    span: Span::NONE,
                }
            }
            1 => MmdBlock::CitationDefinition {
                label: self.word(6),
                content: self.inlines(0),
                span: Span::NONE,
            },
            2 => MmdBlock::ThematicBreak { span: Span::NONE },
            _ => MmdBlock::Paragraph {
                inlines: self.inlines(0),
                span: Span::NONE,
            },
        }
    }

    fn metadata(&mut self) -> (Vec<MetadataEntry>, MetadataStyle) {
        let count = self.byte() % 3;
        if count == 0 {
            return (Vec::new(), MetadataStyle::None);
        }
        let entries = (0..count)
            .map(|_| MetadataEntry {
                key: self.word(6),
                value: self.phrase(2),
            })
            .collect();
        let style = if self.maybe() {
            MetadataStyle::Bare
        } else {
            MetadataStyle::Delimited
        };
        (entries, style)
    }

    fn doc(&mut self) -> MmdDoc {
        let (metadata, metadata_style) = self.metadata();
        let count = (self.byte() % 4) + 1;
        let blocks = (0..count).map(|_| self.block()).collect();
        MmdDoc {
            metadata,
            metadata_style,
            blocks,
            link_defs: Vec::new(),
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let mut g = Gen::new(data);
    let doc = g.doc();

    let emitted = multimarkdown_fmt::emit(&doc);
    let emitted_str = match std::str::from_utf8(&emitted) {
        Ok(s) => s,
        Err(_) => return,
    };

    let (doc2, diags) = multimarkdown_fmt::parse(emitted.as_slice());
    assert!(
        diags.is_empty(),
        "unexpected diagnostics reparsing generated document: {diags:?}\nemitted: {emitted_str}"
    );

    assert_eq!(
        doc.strip_spans(),
        doc2.strip_spans(),
        "multimarkdown-fmt roundtrip mismatch\n  emitted: {emitted_str}"
    );
});
