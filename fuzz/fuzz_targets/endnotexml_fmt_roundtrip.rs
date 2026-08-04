#![no_main]

//! endnotexml-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary EndNoteDoc from fuzz data, emits it to XML
//! bytes, parses back, and asserts structural equality (after
//! strip_spans).
//!
//! Direction: arbitrary_endnotexml_ast → emit → parse → assert equality
//!
//! This is the definitive roundtrip test per CLAUDE.md: starts from the
//! format crate's own Ast type (not the IR).

use endnotexml_fmt::{
    Contributors, Dates, Element, EndNoteDoc, ForeignKey, ForeignKeys, Inline, Periodical, Record,
    RefType, Span, Titles, Urls,
};
use libfuzzer_sys::fuzz_target;

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

    /// Lowercase ASCII letters and spaces only — never empty, never
    /// contains a character that requires XML escaping or could form
    /// markup, and never pure whitespace (so it survives the parser's
    /// implicit whitespace-only-text handling unchanged).
    fn safe_text(&mut self, n: usize) -> String {
        let raw: String = self
            .bytes(n)
            .iter()
            .map(|b| match b % 27 {
                26 => ' ',
                other => (other + b'a') as char,
            })
            .collect();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            "x".to_string()
        } else {
            trimmed.to_string()
        }
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

    fn inline(&mut self, depth: u8) -> Vec<Inline> {
        let count = (self.byte() % 3) + 1;
        (0..count).map(|_| self.inline_item(depth)).collect()
    }

    fn inline_item(&mut self, depth: u8) -> Inline {
        if depth < 2 && self.byte() % 3 == 0 {
            let faces = ["bold", "italic", "underline", "superscript", "subscript"];
            let face = faces[self.byte() as usize % faces.len()].to_string();
            Inline::Style {
                face,
                children: self.inline(depth + 1),
            }
        } else {
            Inline::Text(self.safe_text(5))
        }
    }

    fn maybe_inline(&mut self, depth: u8) -> Option<Vec<Inline>> {
        if self.maybe() {
            Some(self.inline(depth))
        } else {
            None
        }
    }

    fn attrs(&mut self, prefix: char) -> Vec<(String, String)> {
        let count = self.byte() % 2;
        (0..count)
            .map(|i| {
                (
                    format!("{prefix}{}{i}", self.safe_text(3)),
                    self.safe_text(3),
                )
            })
            .collect()
    }

    fn element(&mut self) -> Element {
        Element {
            name: format!("custom{}", self.byte() % 8),
            attrs: self.attrs('x'),
            children: self.inline(1),
        }
    }

    fn author_list(&mut self) -> Vec<Vec<Inline>> {
        if !self.maybe() {
            return Vec::new();
        }
        let count = (self.byte() % 3) + 1;
        (0..count).map(|_| self.inline(1)).collect()
    }

    fn record(&mut self) -> Record {
        let ref_type = RefType {
            code: (self.byte() % 60).to_string(),
            name: self.maybe_text(6),
        };

        let contributors = if self.maybe() {
            Some(Contributors {
                authors: self.author_list(),
                secondary_authors: self.author_list(),
                tertiary_authors: self.author_list(),
                subsidiary_authors: self.author_list(),
                extra: Vec::new(),
            })
        } else {
            None
        };

        let titles = if self.maybe() {
            Some(Titles {
                title: self.maybe_inline(1),
                secondary_title: self.maybe_inline(1),
                tertiary_title: self.maybe_inline(1),
                extra: Vec::new(),
            })
        } else {
            None
        };

        let periodical = if self.maybe() {
            Some(Periodical {
                full_title: self.maybe_inline(1),
                extra: Vec::new(),
            })
        } else {
            None
        };

        let urls = if self.maybe() {
            let related = if self.maybe() {
                vec![self.safe_text(6)]
            } else {
                Vec::new()
            };
            let pdf = if self.maybe() {
                vec![self.safe_text(6)]
            } else {
                Vec::new()
            };
            Some(Urls {
                related_urls: related,
                pdf_urls: pdf,
                extra: Vec::new(),
            })
        } else {
            None
        };

        let foreign_keys = if self.maybe() {
            let count = self.byte() % 2;
            let keys = (0..count)
                .map(|_| ForeignKey {
                    app: self.maybe_text(3),
                    db_id: self.maybe_text(3),
                    text: self.safe_text(4),
                })
                .collect();
            Some(ForeignKeys {
                keys,
                extra: Vec::new(),
            })
        } else {
            None
        };

        let dates = if self.maybe() {
            Some(Dates {
                year: self.maybe_inline(1),
                pub_date: self.maybe_inline(1),
                extra: Vec::new(),
            })
        } else {
            None
        };

        let keyword_count = self.byte() % 3;
        let keywords = (0..keyword_count).map(|_| self.inline(1)).collect();

        let extra_count = self.byte() % 2;
        let extra = (0..extra_count).map(|_| self.element()).collect();

        Record {
            ref_type,
            rec_number: self.maybe_text(3),
            label: self.maybe_text(3),
            foreign_keys,
            contributors,
            titles,
            periodical,
            volume: self.maybe_inline(1),
            number: self.maybe_inline(1),
            pages: self.maybe_inline(1),
            publisher: self.maybe_inline(1),
            pub_location: self.maybe_inline(1),
            isbn: self.maybe_text(4),
            issn: self.maybe_text(4),
            electronic_resource_num: self.maybe_text(6),
            urls,
            bare_url: self.maybe_text(6),
            abstract_: self.maybe_inline(1),
            notes: self.maybe_inline(1),
            keywords,
            dates,
            extra,
            span: Span::NONE,
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let mut g = Gen::new(data);
    let count = (g.byte() % 3) + 1;
    let records: Vec<Record> = (0..count).map(|_| g.record()).collect();

    let doc = EndNoteDoc {
        xml_decl: None,
        records,
        span: Span::NONE,
    };

    // Emit — must not panic.
    let emitted = endnotexml_fmt::emit(&doc);

    // Parse back — must not panic.
    let (doc2, diags) = endnotexml_fmt::parse(&emitted);
    assert!(
        diags.is_empty(),
        "unexpected diagnostics reparsing generated document: {diags:?}\nemitted: {}",
        String::from_utf8_lossy(&emitted)
    );

    assert_eq!(
        doc.strip_spans(),
        doc2.strip_spans(),
        "endnotexml-fmt roundtrip mismatch\n  emitted: {}",
        String::from_utf8_lossy(&emitted)
    );
});
