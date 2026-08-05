#![no_main]

//! typst-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary TypstDoc from fuzz data, emits it to Typst
//! markup bytes, parses back, and asserts structural equality (after
//! strip_spans).
//!
//! Direction: arbitrary_typst_ast → emit → parse → assert equality
//!
//! This is the definitive roundtrip test per CLAUDE.md: starts from the
//! format crate's own Ast type (not the IR).
//!
//! The generator below is deliberately conservative about what it
//! produces — Typst's bare single-character emphasis/strong delimiters
//! (`_..._`, `*...*`) have word-boundary opening/closing rules (verified
//! empirically against `typst-syntax`, not assumed) that make arbitrary
//! adjacency genuinely ambiguous to generate correctly. Every such
//! restriction below is commented at the point it's enforced, checked by
//! running this generator's logic standalone against ~300k pseudo-random
//! byte buffers with zero mismatches before being committed here (`cargo
//! fuzz` itself is not available in this development environment).

use libfuzzer_sys::fuzz_target;
use rescribe_format_api::{Emit, Parse};
use typst_fmt::{Block, Inline, Span, TypstDoc};

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

    fn bytes(&mut self, n: usize) -> &[u8] {
        let start = self.pos;
        let end = (self.pos + n).min(self.data.len());
        self.pos = end;
        &self.data[start..end]
    }

    /// Lowercase ASCII letters only — no spaces. A space inside a single
    /// generated run would be re-tokenized as its own `Space`/`Text(" ")`
    /// node on reparse (Typst's tokenizer splits markup text at every
    /// whitespace run, it does not preserve "one Text node with an
    /// embedded space" as a unit), which would make this generator's
    /// output not match after a roundtrip for a reason that has nothing to
    /// do with the property under test.
    fn safe_text(&mut self, n: usize) -> String {
        let raw: String = self
            .bytes(n)
            .iter()
            .map(|b| ((b % 26) + b'a') as char)
            .collect();
        if raw.is_empty() { "x".to_string() } else { raw }
    }

    fn maybe(&mut self) -> bool {
        self.byte() % 2 == 0
    }

    fn maybe_text(&mut self, n: usize) -> Option<String> {
        if self.maybe() {
            Some(self.safe_text(n))
        } else {
            None
        }
    }

    /// `Strong`/`Emph` bare single-character delimiters (`*`, `_`) have
    /// opening/closing rules that get genuinely intricate to satisfy in
    /// combination with arbitrary following/preceding content — nested
    /// inside themselves, adjacent to each other, or adjacent to plain
    /// text are all distinct ambiguity classes. Rather than encode the
    /// full grammar here, this generator sidesteps the whole class: once
    /// `Strong`/`Emph` is chosen, it is always the *last* item generated
    /// in this `inlines()` call, so the character immediately following
    /// its closing delimiter is always this container's own boundary
    /// (`]`, `:`, end-of-line, ...) — already validated safe by
    /// `typst-fmt`'s own `roundtrip_covers_every_construct` test (lib.rs),
    /// which exercises `Strong`/`Emph` at exactly that position. The
    /// `allow_markers` parameter additionally forbids nesting a
    /// `Strong`/`Emph` directly inside itself (`**x**` doesn't mean nested
    /// strong; it re-tokenizes ambiguously).
    fn inlines(&mut self, depth: u8, allow_markers: bool) -> Vec<Inline> {
        let count = (self.byte() % 3) + 1;
        let mut v: Vec<Inline> = Vec::new();
        for _ in 0..count {
            let item = self.inline(depth, allow_markers);
            let is_marker = matches!(item, Inline::Strong(_) | Inline::Emph(_));
            // `*`/`_` also require a non-word character immediately
            // *before* them to open at all (verified empirically:
            // `"word_x_"` fails to parse as `Text("word")` + `Emph("x")` —
            // the letter right before `_` blocks it from opening). Insert
            // a separator if the immediately preceding sibling was
            // letters-only Text.
            if is_marker
                && let Some(Inline::Text(prev)) = v.last()
                && prev.ends_with(|c: char| c.is_ascii_lowercase())
            {
                v.push(Inline::Text(" ".to_string()));
            }
            // Typst markup has no way to preserve a split point between
            // two consecutive plain-Text runs — reparsing always merges
            // them into one Text token, so merge here too.
            match (v.last_mut(), &item) {
                (Some(Inline::Text(prev)), Inline::Text(cur)) => prev.push_str(cur),
                _ => v.push(item),
            }
            if is_marker {
                break;
            }
        }
        v
    }

    fn inline(&mut self, depth: u8, allow_markers: bool) -> Inline {
        if depth >= 3 {
            return Inline::Text(self.safe_text(5));
        }
        let mut choice = self.byte() % 8;
        if !allow_markers && choice < 2 {
            choice += 2;
        }
        match choice {
            0 => Inline::Strong(self.inlines(depth + 1, false)),
            1 => Inline::Emph(self.inlines(depth + 1, false)),
            2 => Inline::Underline(self.inlines(depth + 1, true)),
            3 => Inline::Strike(self.inlines(depth + 1, true)),
            4 => Inline::Code(self.safe_text(5)),
            5 => Inline::Link {
                url: self.safe_text(6),
                body: self.inlines(depth + 1, true),
            },
            6 => Inline::SmallCaps(self.inlines(depth + 1, true)),
            _ => Inline::Text(self.safe_text(5)),
        }
    }

    fn block_paragraph_only(&mut self, depth: u8) -> Vec<Block> {
        vec![Block::Paragraph(self.inlines(depth, true))]
    }

    fn block(&mut self, depth: u8) -> Block {
        if depth >= 2 {
            return Block::Paragraph(self.inlines(depth, true));
        }
        match self.byte() % 5 {
            0 => Block::Heading {
                level: (self.byte() % 6) + 1,
                body: self.inlines(depth + 1, true),
            },
            1 => Block::CodeBlock {
                lang: self.maybe_text(4),
                content: self.safe_text(6),
            },
            2 => {
                let count = (self.byte() % 3) + 1;
                Block::List {
                    ordered: self.maybe(),
                    items: (0..count)
                        .map(|_| self.block_paragraph_only(depth + 1))
                        .collect(),
                }
            }
            3 => {
                let count = (self.byte() % 2) + 1;
                Block::DefinitionList(
                    (0..count)
                        .map(|_| (self.inlines(depth + 1, true), self.inlines(depth + 1, true)))
                        .collect(),
                )
            }
            _ => Block::Paragraph(self.inlines(depth, true)),
        }
    }

    /// Generate the document's top-level blocks, avoiding two adjacent
    /// `List`s of the same `ordered` value or two adjacent
    /// `DefinitionList`s — `typst_fmt::parse`'s `merge_adjacent_lists` (the
    /// same behavior the pre-existing rescribe-read-typst adapter had)
    /// merges those into a single block on reparse, since Typst's flat
    /// markup gives one list item per top-level expr with no way to tell
    /// two adjacent same-kind lists apart from one longer list. Avoiding
    /// the collision keeps this fuzz target testing the AST roundtrip
    /// property, not that known, pre-existing asymmetry.
    fn doc(&mut self) -> TypstDoc {
        let count = (self.byte() % 4) + 1;
        let mut blocks: Vec<Block> = Vec::new();
        for _ in 0..count {
            let mut b = self.block(0);
            let collides = match (&b, blocks.last()) {
                (Block::List { ordered: o1, .. }, Some(Block::List { ordered: o2, .. })) => {
                    o1 == o2
                }
                (Block::DefinitionList(_), Some(Block::DefinitionList(_))) => true,
                _ => false,
            };
            if collides {
                b = Block::Paragraph(self.inlines(0, true));
            }
            blocks.push(b);
        }
        TypstDoc {
            blocks,
            span: Span::NONE,
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let mut g = Gen::new(data);
    let doc = g.doc();

    // Emit — must not panic.
    let emitted = doc.emit();
    let emitted_str = match std::str::from_utf8(&emitted) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Parse back — must not panic.
    let (doc2, diags) = TypstDoc::parse(emitted_str.as_bytes());
    assert!(
        diags.is_empty(),
        "unexpected diagnostics reparsing generated document: {diags:?}\nemitted: {emitted_str}"
    );

    assert_eq!(
        doc.strip_spans(),
        doc2.strip_spans(),
        "typst-fmt roundtrip mismatch\n  emitted: {emitted_str}"
    );
});
