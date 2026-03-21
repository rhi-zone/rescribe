#![no_main]
//! MediaWiki roundtrip fuzz target.
//!
//! Generates an arbitrary MediawikiDoc (subset: Paragraph, Heading, HorizontalRule),
//! emits it to MediaWiki markup, parses it back, strips spans, and asserts equality.
//!
//! Direction: arbitrary MediawikiDoc → emit → parse → strip_spans → assert equal

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use mediawiki_fmt::{Block, Inline, MediawikiDoc, Span};

#[derive(Arbitrary, Debug)]
enum FuzzBlock {
    Para { text: String },
    Heading { level: u8, text: String },
    Hr,
}

/// Sanitise text for paragraph or heading content.
///
/// Strip characters that would be re-parsed as markup or structural elements:
/// - `'`  → bold/italic markers
/// - `[`, `]` → links
/// - `{`, `}` → templates/tables
/// - `|`, `!` → table cell markers
/// - `#`, `*` → list markers (only problematic at line start, but sanitise globally for simplicity)
/// - `=` → heading markers
/// - `<`, `>` → HTML tags
/// - `\x00..=\x1f` → control characters
/// - leading space → code block
fn sanitise(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(*c, '\'' | '[' | ']' | '{' | '}' | '|' | '!' | '#' | '*' | '=' | '<' | '>' | '\x00'..='\x1f'))
        .collect::<String>()
        .trim()
        .to_string()
}

fuzz_target!(|blocks: Vec<FuzzBlock>| {
    let doc_blocks: Vec<Block> = blocks
        .into_iter()
        .filter_map(|b| match b {
            FuzzBlock::Para { text } => {
                let clean = sanitise(&text);
                if clean.is_empty() {
                    return None;
                }
                // Also reject if it starts with '-' sequences that could look like hr
                // (4+ dashes all the same char triggers hr detection in parser)
                let all_dashes = clean.chars().all(|c| c == '-');
                if all_dashes && clean.len() >= 4 {
                    return None;
                }
                Some(Block::Paragraph {
                    inlines: vec![Inline::Text(clean)],
                    span: Span::NONE,
                })
            }
            FuzzBlock::Heading { level, text } => {
                let level = (level % 6) + 1;
                let clean = sanitise(&text);
                if clean.is_empty() {
                    return None;
                }
                Some(Block::Heading { level, inlines: vec![Inline::Text(clean)], span: Span::NONE })
            }
            FuzzBlock::Hr => Some(Block::HorizontalRule),
        })
        .collect();

    if doc_blocks.is_empty() {
        return;
    }

    let doc = MediawikiDoc { blocks: doc_blocks, span: Span::NONE };

    let wiki_text = mediawiki_fmt::emit(&doc);

    let (parsed, _diags) = mediawiki_fmt::parse(&wiki_text);
    let parsed = parsed.strip_spans();
    let expected = doc.strip_spans();

    assert_eq!(
        expected,
        parsed,
        "MediaWiki roundtrip mismatch\n  emitted: {wiki_text:?}"
    );
});
