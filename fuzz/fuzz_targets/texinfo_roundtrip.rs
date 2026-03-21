#![no_main]
//! Texinfo roundtrip fuzz target.
//!
//! Generates arbitrary TexinfoDocs (paragraphs + headings + code blocks),
//! emits them, parses back, strips spans, asserts equality.
//!
//! Direction: arbitrary TexinfoDoc → emit → parse → strip_spans → assert equal
//!
//! Characters stripped from paragraph text: `@`, `{`, `}`, control chars.
//! Code block content strips only null bytes.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use texinfo::{Block, Inline, Span, TexinfoDoc};

#[derive(Arbitrary, Debug)]
struct FuzzPara {
    text: String,
}

#[derive(Arbitrary, Debug)]
struct FuzzHeading {
    level: u8,
    text: String,
}

#[derive(Arbitrary, Debug)]
struct FuzzCode {
    content: String,
}

#[derive(Arbitrary, Debug)]
enum FuzzBlock {
    Para(FuzzPara),
    Heading(FuzzHeading),
    Code(FuzzCode),
}

fn sanitise_text(s: &str) -> String {
    // Strip characters that would be re-interpreted as Texinfo commands or structure.
    // Backslash is stripped because \input, \end etc. are TeX directives that the
    // Texinfo parser may handle specially.
    s.chars()
        .filter(|c| !matches!(*c, '\x00'..='\x1f' | '@' | '{' | '}' | '\\'))
        .collect::<String>()
        .trim()
        .to_string()
}

fn sanitise_code(s: &str) -> String {
    // Code blocks end at @end example — strip @ and \r to avoid that.
    // Newlines are allowed but strip leading/trailing to avoid normalization
    // differences (a "\n"-only code block round-trips as "").
    let s: String = s.chars().filter(|c| !matches!(*c, '\x00' | '@' | '\r')).collect();
    s.trim_matches('\n').to_string()
}

fuzz_target!(|blocks: Vec<FuzzBlock>| {
    let doc_blocks: Vec<Block> = blocks
        .into_iter()
        .filter_map(|b| match b {
            FuzzBlock::Para(p) => {
                let text = sanitise_text(&p.text);
                if text.is_empty() {
                    None
                } else {
                    Some(Block::Paragraph {
                        inlines: vec![Inline::Text(text, Span::NONE)],
                        span: Span::NONE,
                    })
                }
            }
            FuzzBlock::Heading(h) => {
                let text = sanitise_text(&h.text);
                if text.is_empty() {
                    None
                } else {
                    let level = (h.level % 4) + 1;
                    Some(Block::Heading {
                        level,
                        inlines: vec![Inline::Text(text, Span::NONE)],
                        span: Span::NONE,
                    })
                }
            }
            FuzzBlock::Code(c) => {
                let content = sanitise_code(&c.content);
                Some(Block::CodeBlock { content, span: Span::NONE })
            }
        })
        .collect();

    if doc_blocks.is_empty() {
        return;
    }

    let doc = TexinfoDoc { title: None, blocks: doc_blocks, span: Span::NONE };

    let texinfo_text = texinfo::emit(&doc);
    let (parsed, _diags) = texinfo::parse(&texinfo_text);

    let expected = doc.strip_spans();
    let got = parsed.strip_spans();

    assert_eq!(
        expected,
        got,
        "Texinfo roundtrip mismatch\n  emitted:\n{texinfo_text}"
    );
});
