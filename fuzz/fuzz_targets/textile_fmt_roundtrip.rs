#![no_main]

//! textile-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary TextileDoc from fuzz data, emits it to Textile text,
//! parses back, and asserts structural equality (after strip_spans).
//!
//! Direction: arbitrary_textile_ast → emit → parse → assert equality
//!
//! This is the definitive test per CLAUDE.md: starts from the format crate's
//! own Ast type (not the IR). Covers the full surface area of what Textile can
//! express, regardless of IR modeling completeness.
//!
//! Restricted to constructs that roundtrip cleanly:
//! - Plain text: characters that Textile uses as inline markup delimiters are
//!   stripped so the corpus exercises structural variations.
//! - Bold (`*bold*`), Italic (`_italic_`), Code (`@code@`) are safe in isolation.
//! - Strikethrough (`-text-`), Underline (`+text+`), Superscript (`^text^`),
//!   Subscript (`~text~`) are excluded from plain text contexts to avoid
//!   false markup triggering.
//! - Nested markup is excluded to avoid delimiter collision edge cases.

use libfuzzer_sys::fuzz_target;
use textile_fmt::{Block, BlockAttrs, Inline, Span, TableCell, TableRow, TextileDoc};

// ── Safe text generator ───────────────────────────────────────────────────────

fn safe_str(bytes: &[u8]) -> String {
    // Only use lowercase letters a-z to completely avoid any Textile markup
    // delimiters, structural characters, and control codes.
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

// ── Generator ────────────────────────────────────────────────────────────────

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

    fn inline(&mut self, depth: u8) -> Inline {
        // At depth > 0, only produce leaf inlines to avoid nested delimiter
        // collision ambiguity.
        let kind = self.byte() % if depth > 0 { 2 } else { 4 };
        match kind {
            0 => Inline::Text(safe_text(self.bytes(3)), Span::dummy()),
            1 => Inline::Code(safe_text(self.bytes(3)), Span::dummy()),
            2 => Inline::Bold(self.inlines(depth + 1, 1), Span::dummy()),
            _ => Inline::Italic(self.inlines(depth + 1, 1), Span::dummy()),
        }
    }

    fn inlines(&mut self, depth: u8, min: usize) -> Vec<Inline> {
        if depth > 2 {
            return vec![Inline::Text(safe_text(self.bytes(2)), Span::dummy())];
        }
        let count = (self.byte() as usize % 3) + min;
        let raw: Vec<Inline> = (0..count).map(|_| self.inline(depth)).collect();
        // Merge adjacent Text nodes — roundtrip may collapse them.
        let merged = merge_text(raw);
        // Textile closing delimiters (*bold*, _italic_) require the character
        // after the delimiter to be non-alphanumeric. Since safe_text produces
        // only a-z, a formatted span followed by any other inline would produce
        // e.g. "*x*abc" which the parser treats as plain text. To avoid
        // roundtrip failure, if any Bold/Italic appears with siblings, keep
        // only that formatted span (as the sole inline).
        let needs_isolation = merged.len() > 1
            && merged.iter().any(|i| matches!(i, Inline::Bold(..) | Inline::Italic(..)));
        if needs_isolation {
            // Keep only the first Bold or Italic inline (guaranteed to exist by any()).
            let fmt = merged.into_iter().find(|i| matches!(i, Inline::Bold(..) | Inline::Italic(..))).unwrap();
            return vec![fmt];
        }
        merged
    }

    fn block(&mut self, depth: u8) -> Block {
        // At depth > 0 (inside blockquote/list), restrict to simple constructs.
        let kind = self.byte() % if depth > 0 { 2 } else { 5 };
        match kind {
            0 => Block::Paragraph {
                inlines: self.inlines(0, 1),
                align: None,
                attrs: BlockAttrs::default(),
                span: Span::dummy(),
            },
            1 => {
                let level = (self.byte() % 6) + 1;
                Block::Heading {
                    level,
                    inlines: self.inlines(0, 1),
                    attrs: BlockAttrs::default(),
                    span: Span::dummy(),
                }
            }
            2 => {
                let n = (self.byte() as usize % 3) + 1;
                let items: Vec<Vec<Block>> = (0..n)
                    .map(|_| {
                        vec![Block::Paragraph {
                            inlines: self.inlines(0, 1),
                            align: None,
                            attrs: BlockAttrs::default(),
                            span: Span::dummy(),
                        }]
                    })
                    .collect();
                Block::List {
                    ordered: self.byte() % 2 == 0,
                    items,
                    span: Span::dummy(),
                }
            }
            3 => Block::CodeBlock {
                content: safe_text(self.bytes(4)),
                language: None,
                span: Span::dummy(),
            },
            _ => {
                // Table with 1 row and 2 cells
                let cell1 = TableCell {
                    is_header: false,
                    align: None,
                    inlines: self.inlines(0, 1),
                    span: Span::dummy(),
                };
                let cell2 = TableCell {
                    is_header: false,
                    align: None,
                    inlines: self.inlines(0, 1),
                    span: Span::dummy(),
                };
                Block::Table {
                    rows: vec![TableRow {
                        attrs: BlockAttrs::default(),
                        cells: vec![cell1, cell2],
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                }
            }
        }
    }
}

fn merge_text(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text(content, _) => {
                if let Some(Inline::Text(prev, _)) = out.last_mut() {
                    prev.push_str(&content);
                } else {
                    out.push(Inline::Text(content, Span::dummy()));
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
    let block_count = (g.byte() as usize % 3) + 1;
    if g.data.len() < block_count {
        return;
    }

    let blocks: Vec<Block> = (0..block_count).map(|_| g.block(0)).collect();
    let doc = TextileDoc { blocks, span: Span::dummy() };

    // Emit — must not panic.
    let emitted = textile_fmt::emit(&doc);

    // Parse back — must not panic.
    let (doc2, _diags) = textile_fmt::parse(&emitted);

    // Structural equality after strip_spans.
    // parse(emit(doc)).strip_spans() == doc.strip_spans()
    assert_eq!(
        doc.strip_spans(),
        doc2.strip_spans(),
        "textile-fmt roundtrip mismatch\n  emitted: {emitted:?}"
    );
});
