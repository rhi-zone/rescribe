#![no_main]

//! opml-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary OpmlDoc from fuzz data, emits it to XML bytes,
//! parses back, and asserts structural equality (after strip_spans).
//!
//! Direction: arbitrary_opml_ast → emit → parse → assert equality
//!
//! This is the definitive roundtrip test per CLAUDE.md: starts from the
//! format crate's own Ast type (not the IR).

use libfuzzer_sys::fuzz_target;
use opml_fmt::{Body, Head, OpmlDoc, Outline, Span};
use rescribe_format_api::{Emit as _, Parse as _};

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

    /// Lowercase ASCII letters only — never empty, never contains any
    /// character that requires XML escaping or could form markup.
    fn safe_text(&mut self, n: usize) -> String {
        let raw: String = self
            .bytes(n)
            .iter()
            .map(|b| ((*b % 26) + b'a') as char)
            .collect();
        if raw.is_empty() { "x".to_string() } else { raw }
    }

    fn maybe_text(&mut self, n: usize) -> Option<String> {
        if self.byte() % 2 == 0 {
            None
        } else {
            Some(self.safe_text(n))
        }
    }

    fn attrs(&mut self, prefix: char) -> Vec<(String, String)> {
        let count = self.byte() % 3;
        // Attribute names include their index so two attrs on the same
        // outline can never collide.
        (0..count)
            .map(|i| {
                (
                    format!("{prefix}{}{i}", self.safe_text(3)),
                    self.safe_text(3),
                )
            })
            .collect()
    }

    fn outline(&mut self, depth: u8) -> Outline {
        let children = if depth < 3 && self.byte() % 2 == 0 {
            let count = (self.byte() % 3) + 1;
            (0..count).map(|_| self.outline(depth + 1)).collect()
        } else {
            Vec::new()
        };
        let self_closing = children.is_empty() && self.byte() % 2 == 0;
        Outline {
            attrs: self.attrs('a'),
            children,
            self_closing,
            span: Span::NONE,
        }
    }

    fn head(&mut self) -> Head {
        Head {
            title: self.maybe_text(4),
            date_created: self.maybe_text(4),
            date_modified: self.maybe_text(4),
            owner_name: self.maybe_text(4),
            owner_email: self.maybe_text(4),
            owner_id: self.maybe_text(4),
            docs: self.maybe_text(4),
            expansion_state: self.maybe_text(4),
            vert_scroll_state: self.maybe_text(4),
            window_top: self.maybe_text(3),
            window_left: self.maybe_text(3),
            window_bottom: self.maybe_text(3),
            window_right: self.maybe_text(3),
            extra: self.attrs('h'),
            span: Span::NONE,
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let mut g = Gen::new(data);

    let count = (g.byte() % 4) + 1;
    let outlines: Vec<Outline> = (0..count).map(|_| g.outline(0)).collect();

    let doc = OpmlDoc {
        xml_decl: None,
        version: "2.0".to_string(),
        head: g.head(),
        body: Body {
            outlines,
            span: Span::NONE,
        },
        span: Span::NONE,
    };

    // Emit — must not panic.
    let emitted = doc.emit();

    // Parse back — must not panic.
    let (doc2, diags) = OpmlDoc::parse(&emitted);
    assert!(
        diags.is_empty(),
        "unexpected diagnostics reparsing generated document: {diags:?}\nemitted: {}",
        String::from_utf8_lossy(&emitted)
    );

    assert_eq!(
        doc.strip_spans(),
        doc2.strip_spans(),
        "opml-fmt roundtrip mismatch\n  emitted: {}",
        String::from_utf8_lossy(&emitted)
    );
});
