#![no_main]

//! docbook-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary DocBookDoc from fuzz data, emits it to XML bytes,
//! parses back, and asserts structural equality (after strip_spans).
//!
//! Direction: arbitrary_docbook_ast → emit → parse → assert equality
//!
//! This is the definitive roundtrip test per CLAUDE.md: starts from the
//! format crate's own Ast type (not the IR). Covers the full surface area of
//! what generic XML can express, regardless of DocBook-specific IR modeling.

use docbook_fmt::{DocBookDoc, Node};
use libfuzzer_sys::fuzz_target;
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
    /// character that requires XML escaping or could form markup (`<`, `&`,
    /// `"`, `-`, `]`, whitespace).
    fn safe_text(&mut self, n: usize) -> String {
        let raw: String = self
            .bytes(n)
            .iter()
            .map(|b| ((*b % 26) + b'a') as char)
            .collect();
        if raw.is_empty() { "x".to_string() } else { raw }
    }

    /// A valid XML Name fragment: starts with a fixed letter so it can never
    /// collide with reserved prefixes (`xml*`) or predefined entity names.
    fn safe_name(&mut self, prefix: char, n: usize) -> String {
        format!("{prefix}{}", self.safe_text(n))
    }

    fn attrs(&mut self) -> Vec<(String, String)> {
        let count = self.byte() % 3;
        // Attribute names include their index so two attrs on the same
        // element can never collide — duplicate attribute names make the
        // emitted XML non-well-formed (quick-xml correctly rejects it),
        // which is a fuzz-generator bug, not a library bug.
        (0..count)
            .map(|i| (format!("{}{i}", self.safe_name('a', 3)), self.safe_text(3)))
            .collect()
    }

    /// A leaf node that is never `Text` — used to break up runs of text so
    /// two adjacent `Text` nodes (which the parser would merge back into
    /// one) never appear in the generated tree.
    fn non_text_leaf(&mut self) -> Node {
        match self.byte() % 3 {
            0 => Node::Comment {
                content: self.safe_text(4),
                span: docbook_fmt::Span::NONE,
            },
            1 => Node::Cdata {
                content: self.safe_text(4),
                span: docbook_fmt::Span::NONE,
            },
            _ => Node::EntityRef {
                name: self.safe_name('e', 3),
                span: docbook_fmt::Span::NONE,
            },
        }
    }

    fn node(&mut self, depth: u8) -> Node {
        if depth > 2 {
            return Node::Text {
                content: self.safe_text(3),
                span: docbook_fmt::Span::NONE,
            };
        }
        match self.byte() % 5 {
            0 => Node::Text {
                content: self.safe_text(3),
                span: docbook_fmt::Span::NONE,
            },
            1 | 2 => Node::Element {
                name: self.safe_name('n', 4),
                attrs: self.attrs(),
                children: self.children(depth + 1),
                span: docbook_fmt::Span::NONE,
            },
            _ => self.non_text_leaf(),
        }
    }

    fn children(&mut self, depth: u8) -> Vec<Node> {
        let count = (self.byte() % 3) + 1;
        let mut out: Vec<Node> = Vec::new();
        for _ in 0..count {
            let mut n = self.node(depth);
            // Never allow two adjacent Text nodes — the parser coalesces
            // consecutive text runs into a single node, so generating two
            // in a row would make the "expected" tree diverge from what
            // parse() actually produces.
            if matches!(n, Node::Text { .. }) && matches!(out.last(), Some(Node::Text { .. })) {
                n = self.non_text_leaf();
            }
            out.push(n);
        }
        out
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let mut g = Gen::new(data);

    let mut nodes: Vec<Node> = Vec::new();
    if g.byte() % 2 == 0 {
        nodes.push(Node::Doctype {
            content: g.safe_text(6),
            span: docbook_fmt::Span::NONE,
        });
    }
    if g.byte() % 2 == 0 {
        nodes.push(Node::ProcessingInstruction {
            target: g.safe_name('p', 3),
            data: g.safe_text(3),
            span: docbook_fmt::Span::NONE,
        });
    }
    nodes.push(Node::Element {
        name: g.safe_name('n', 4),
        attrs: g.attrs(),
        children: g.children(0),
        span: docbook_fmt::Span::NONE,
    });

    let doc = DocBookDoc {
        xml_decl: None,
        nodes,
    };

    // Emit — must not panic.
    let emitted = doc.emit();

    // Parse back — must not panic.
    let (doc2, diags) = DocBookDoc::parse(&emitted);
    assert!(
        diags.is_empty(),
        "unexpected diagnostics reparsing generated document: {diags:?}\nemitted: {}",
        String::from_utf8_lossy(&emitted)
    );

    assert_eq!(
        doc.strip_spans(),
        doc2.strip_spans(),
        "docbook-fmt roundtrip mismatch\n  emitted: {}",
        String::from_utf8_lossy(&emitted)
    );
});
