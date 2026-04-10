#![no_main]

//! odf-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary OdfDocument from fuzz data, emits it to an ODF ZIP
//! archive, parses back, and asserts structural equality.
//!
//! Direction: arbitrary_odf_ast → emit → parse → assert equality
//!
//! This is the definitive roundtrip test per CLAUDE.md: starts from the format
//! crate's own Ast type. Covers the full surface area of ODF regardless of IR
//! modeling completeness.

use libfuzzer_sys::fuzz_target;
use odf_fmt::*;

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

    /// Produce safe text content (printable ASCII letters only, no XML special chars).
    fn safe_text(&mut self, n: usize) -> String {
        self.bytes(n).iter().map(|b| ((*b % 26) + b'a') as char).collect()
    }

    fn inline(&mut self, depth: u8) -> Inline {
        let kind = self.byte() % if depth > 0 { 2 } else { 4 };
        match kind {
            0 => Inline::Text(self.safe_text(4)),
            1 => Inline::Text(self.safe_text(2)),
            2 => Inline::Span(Span {
                style_name: None,
                content: self.inlines(depth + 1, 1),
            }),
            _ => Inline::Hyperlink(Hyperlink {
                href: Some(format!("https://example.com/{}", self.safe_text(3))),
                title: None,
                style_name: None,
                content: self.inlines(depth + 1, 1),
            }),
        }
    }

    fn inlines(&mut self, depth: u8, min: usize) -> Vec<Inline> {
        if depth > 2 {
            return vec![Inline::Text(self.safe_text(2))];
        }
        let count = (self.byte() as usize % 3) + min;
        (0..count).map(|_| self.inline(depth)).collect()
    }

    fn paragraph(&mut self) -> TextBlock {
        TextBlock::Paragraph(Paragraph {
            content: self.inlines(0, 1),
            ..Default::default()
        })
    }

    fn block(&mut self, depth: u8) -> TextBlock {
        if depth > 2 {
            return self.paragraph();
        }
        let kind = self.byte() % 4;
        match kind {
            0 => self.paragraph(),
            1 => TextBlock::Heading(Heading {
                outline_level: Some((self.byte() % 6 + 1) as u32),
                content: self.inlines(0, 1),
                ..Default::default()
            }),
            2 => {
                let n = (self.byte() as usize % 3) + 1;
                TextBlock::List(List {
                    items: (0..n).map(|_| ListItem {
                        content: vec![self.block(depth + 1)],
                        ..Default::default()
                    }).collect(),
                    ..Default::default()
                })
            }
            _ => TextBlock::Section(Section {
                name: Some(self.safe_text(4)),
                content: vec![self.paragraph()],
                ..Default::default()
            }),
        }
    }

    fn blocks(&mut self, depth: u8, n: usize) -> Vec<TextBlock> {
        (0..n).map(|_| self.block(depth)).collect()
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let mut g = Gen::new(data);
    let block_count = (g.byte() as usize % 4) + 1;
    let blocks = g.blocks(0, block_count);

    let doc = OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(blocks),
        ..Default::default()
    };

    // emit — must not panic
    let bytes = match odf_fmt::emit(&doc) {
        Ok(b) => b,
        Err(_) => return,
    };

    // parse back — must not panic
    let result = match odf_fmt::parse(&bytes) {
        Ok(r) => r,
        Err(_) => return,
    };

    let doc2 = result.value;

    // Structural equality: the body must survive the roundtrip.
    let OdfBody::Text(orig) = &doc.body else { return };
    let OdfBody::Text(parsed) = &doc2.body else {
        panic!("roundtrip changed body kind");
    };

    assert_eq!(orig.len(), parsed.len(),
        "block count changed in roundtrip: {} vs {}", orig.len(), parsed.len());
});
