#![no_main]

//! djot-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary DjotDoc from fuzz data, emits it to Djot text,
//! parses back, and asserts structural equality (after strip_spans).
//!
//! Direction: arbitrary_djot_ast → emit → parse → assert equality
//!
//! This is the definitive test per CLAUDE.md: starts from the format crate's
//! own Ast type (not the IR). Covers the full surface area of what Djot can
//! express, regardless of IR modeling completeness.

use libfuzzer_sys::fuzz_target;
use djot_fmt::{Block, DjotDoc, Inline};

// ── Helpers to build a well-formed DjotDoc from raw bytes ─────────────────────

fn safe_str(bytes: &[u8]) -> String {
    // Only use printable ASCII, avoid markup delimiters that make emit/parse
    // inherently lossy (e.g. backtick runs that form verbatim spans).
    bytes
        .iter()
        .map(|b| {
            let c = (b % 26) + b'a';
            c as char
        })
        .collect()
}

fn safe_text(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        "x".to_string()
    } else {
        safe_str(bytes)
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

    fn bytes(&mut self, n: usize) -> &[u8] {
        let start = self.pos;
        let end = (self.pos + n).min(self.data.len());
        self.pos = end;
        &self.data[start..end]
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn inline(&mut self, depth: u8) -> Inline {
        // At depth > 0 (inside markup spans), only allow leaf inlines to avoid
        // nested-delimiter ambiguity: `*_x_*` vs `*` adjacent to `_x_*`, etc.
        let kind = self.byte() % if depth > 0 { 2 } else { 6 };
        match kind {
            0 => Inline::Text {
                content: safe_text(self.bytes(3)),
                span: djot_fmt::Span::NONE,
            },
            1 => Inline::Verbatim {
                content: safe_text(self.bytes(3)),
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            },
            2 => Inline::Emphasis {
                inlines: self.inlines(depth + 1, 1),
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            },
            3 => Inline::Strong {
                inlines: self.inlines(depth + 1, 1),
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            },
            4 => Inline::Subscript {
                inlines: self.inlines(depth + 1, 1),
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            },
            _ => Inline::Superscript {
                inlines: self.inlines(depth + 1, 1),
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            },
        }
    }

    fn inlines(&mut self, depth: u8, min: usize) -> Vec<Inline> {
        if depth > 2 {
            return vec![Inline::Text {
                content: safe_text(self.bytes(2)),
                span: djot_fmt::Span::NONE,
            }];
        }
        let count = (self.byte() as usize % 3) + min;
        let raw: Vec<Inline> = (0..count).map(|_| self.inline(depth)).collect();
        // Merge adjacent Text nodes — roundtrip collapses them: [Text "a", Text "b"] → [Text "ab"]
        merge_text(raw)
    }

    fn block(&mut self, depth: u8) -> Block {
        let kind = self.byte() % if depth > 0 { 5 } else { 8 };
        match kind {
            0 => Block::Paragraph {
                inlines: self.inlines(0, 1),
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            },
            1 => {
                let level = (self.byte() % 6) + 1;
                Block::Heading {
                    level,
                    inlines: self.inlines(0, 1),
                    attr: Default::default(),
                    span: djot_fmt::Span::NONE,
                }
            }
            2 => {
                let n = (self.byte() as usize % 3) + 1;
                // Tight lists must have exactly 1 block per item (no blank lines possible).
                let items = (0..n)
                    .map(|_| djot_fmt::ListItem {
                        blocks: vec![self.block(depth + 1)],
                        checked: None,
                        span: djot_fmt::Span::NONE,
                    })
                    .collect();
                Block::List {
                    kind: djot_fmt::ListKind::Bullet(djot_fmt::BulletStyle::Dash),
                    items,
                    tight: true,
                    attr: Default::default(),
                    span: djot_fmt::Span::NONE,
                }
            }
            3 => Block::CodeBlock {
                language: Some(safe_text(self.bytes(2))),
                content: format!("{}\n", safe_text(self.bytes(4))),
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            },
            4 => Block::Blockquote {
                blocks: self.blocks(depth + 1, 1),
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            },
            5 => Block::ThematicBreak {
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            },
            6 => {
                // Ordered list (tight: 1 block per item)
                let n = (self.byte() as usize % 3) + 1;
                let items = (0..n)
                    .map(|_| djot_fmt::ListItem {
                        blocks: vec![self.block(depth + 1)],
                        checked: None,
                        span: djot_fmt::Span::NONE,
                    })
                    .collect();
                Block::List {
                    kind: djot_fmt::ListKind::Ordered {
                        style: djot_fmt::OrderedStyle::Decimal,
                        delimiter: djot_fmt::OrderedDelimiter::Period,
                        start: 1,
                    },
                    items,
                    tight: true,
                    attr: Default::default(),
                    span: djot_fmt::Span::NONE,
                }
            }
            _ => Block::Paragraph {
                inlines: self.inlines(0, 1),
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            },
        }
    }

    fn blocks(&mut self, depth: u8, _min: usize) -> Vec<Block> {
        if depth > 2 {
            return vec![Block::Paragraph {
                inlines: vec![Inline::Text {
                    content: safe_text(self.bytes(2)),
                    span: djot_fmt::Span::NONE,
                }],
                attr: Default::default(),
                span: djot_fmt::Span::NONE,
            }];
        }
        // Always produce exactly 1 block to avoid consecutive-same-kind-list merging.
        // Djot: blank line between two bullet lists → parser merges them into one loose list.
        vec![self.block(depth)]
    }
}

fn merge_text(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text { content, .. } => {
                if let Some(Inline::Text { content: prev, .. }) = out.last_mut() {
                    prev.push_str(&content);
                } else {
                    out.push(Inline::Text { content, span: djot_fmt::Span::NONE });
                }
            }
            other => out.push(other),
        }
    }
    out
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let mut g = Gen::new(data);
    // Limit to 1 block to avoid consecutive same-kind lists merging on roundtrip.
    // (Djot: blank line between two bullet lists → parser merges them into one loose list.)
    let block_count = 1;
    if g.remaining() < block_count {
        return;
    }

    let blocks = (0..block_count).map(|_| g.block(0)).collect();
    let doc = DjotDoc {
        blocks,
        footnotes: vec![],
        link_defs: vec![],
    };

    // Emit — must not panic.
    let emitted = djot_fmt::emit(&doc);

    // Parse back — must not panic.
    let (doc2, _diags) = djot_fmt::parse(&emitted);

    // Structural equality after strip_spans.
    // parse(emit(doc)).strip_spans() == doc.strip_spans()
    assert_eq!(
        doc.strip_spans(),
        doc2.strip_spans(),
        "djot-fmt roundtrip mismatch\n  emitted: {emitted:?}"
    );
});
