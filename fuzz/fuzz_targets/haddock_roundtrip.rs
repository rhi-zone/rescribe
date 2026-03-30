#![no_main]

//! Haddock roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with Haddock-compatible constructs,
//! emits them to Haddock text via rescribe-write-haddock, parses them back via
//! rescribe-read-haddock, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: Haddock uses `__bold__` `/italic/` `@code@`
//! `"text"<url>` `[term]` for inline markup, and `=`-prefixed headings,
//! `*` / `(n)` for lists. Characters stripped:
//! - ASCII control chars 0x00–0x1f (includes newlines)
//! - `_` — bold delimiter (__...__); also used in identifiers but ambiguous
//! - `/` — italic delimiter (/.../)
//! - `@` — code delimiter (@...@)
//! - `<` `>` — link syntax
//! - `"` — link text delimiter
//! - `[` `]` — definition list term markers
//! - `*` — unordered list marker
//! - `=` — heading marker (at line start, strip everywhere)
//! - `(` `)` — ordered list marker e.g. (1)

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rescribe_core::{Document, Node};
use rescribe_std::{node, prop};

// ── Fuzz-friendly inline types ────────────────────────────────────────────────

#[derive(Arbitrary, Debug)]
enum FuzzInlineKind {
    Plain,
    Bold,
    // Italic excluded: Haddock requires a non-alphanumeric char before /italic/.
    // When italic follows a word or number (e.g. "998/word/"), the parser treats
    // the "/" as literal text. This is a format constraint, not a bug.
    Code,
}

#[derive(Arbitrary, Debug)]
struct FuzzInline {
    text: String,
    kind: FuzzInlineKind,
}

// ── Fuzz-friendly block types ─────────────────────────────────────────────────

#[derive(Arbitrary, Debug)]
enum FuzzBlock {
    Paragraph { inlines: Vec<FuzzInline> },
    Heading { level: u8, inlines: Vec<FuzzInline> },
    BulletList { items: Vec<Vec<FuzzInline>> },
    OrderedList { items: Vec<Vec<FuzzInline>> },
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip characters that would be re-interpreted as Haddock markup.
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\x00'..='\x1f'
                    // DEL (0x7f): control character that can break @code@ span parsing
                    | '\x7f'
                    | '_' | '/' | '@' | '<' | '>' | '"' | '\'' | '[' | ']' | '*' | '=' | '(' | ')'
                    // Backtick: module identifier / code fence in Haddock
                    | '`'
                    // Anchor syntax: #anchor# — # inside @code@ breaks delimiter parsing
                    | '#'
                    // Template variable syntax: $name
                    | '$'
            )
        })
        .collect();

    let mut out = out.trim().to_string();

    // Strip ordered list patterns (digit+. or digit+) ) that could be re-parsed
    // as ordered list markers.
    loop {
        let prev = out.clone();
        out = out.trim().to_string();
        loop {
            let trimmed = out.trim_start_matches(|c: char| c.is_ascii_digit()).to_string();
            if trimmed.starts_with(". ") {
                let cut = out.len() - trimmed.len() + 2;
                out = out[cut..].trim().to_string();
            } else if trimmed == "." {
                out = String::new();
                break;
            } else {
                break;
            }
        }
        if out == prev {
            break;
        }
    }

    let out = out.trim().to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn make_inline(fi: &FuzzInline) -> Option<Node> {
    let text = sanitise(&fi.text)?;
    // CODE inlines emit as @content@. If content starts with a letter, Haddock
    // may parse @word as a paragraph-level annotation (@deprecated, @param, etc.).
    // Skip such CODE spans to avoid ambiguity with annotation syntax.
    if matches!(fi.kind, FuzzInlineKind::Code)
        && text.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
    {
        return None;
    }
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, text.clone());
    Some(match fi.kind {
        FuzzInlineKind::Plain => leaf,
        FuzzInlineKind::Bold => Node::new(node::STRONG).child(leaf),
        FuzzInlineKind::Code => Node::new(node::CODE).prop(prop::CONTENT, text),
    })
}

fn make_para_children(inlines: &[FuzzInline]) -> Vec<Node> {
    inlines.iter().filter_map(make_inline).collect()
}

fn make_list_item(inlines: &[FuzzInline]) -> Option<Node> {
    let children = make_para_children(inlines);
    if children.is_empty() {
        return None;
    }
    let para = Node::new(node::PARAGRAPH).children(children);
    Some(Node::new(node::LIST_ITEM).child(para))
}

// ── Fuzz target ───────────────────────────────────────────────────────────────

fuzz_target!(|blocks: Vec<FuzzBlock>| {
    let content_nodes: Vec<Node> = blocks
        .iter()
        .filter_map(|b| match b {
            FuzzBlock::Paragraph { inlines } => {
                let children = make_para_children(inlines);
                if children.is_empty() {
                    None
                } else {
                    Some(Node::new(node::PARAGRAPH).children(children))
                }
            }
            FuzzBlock::Heading { level, inlines } => {
                let children = make_para_children(inlines);
                if children.is_empty() {
                    return None;
                }
                let lvl = i64::from(*level % 6) + 1; // 1–6
                Some(Node::new(node::HEADING).prop(prop::LEVEL, lvl).children(children))
            }
            FuzzBlock::BulletList { items } => {
                let list_items: Vec<Node> =
                    items.iter().filter_map(|i| make_list_item(i)).collect();
                if list_items.is_empty() {
                    None
                } else {
                    Some(
                        Node::new(node::LIST)
                            .prop(prop::ORDERED, false)
                            .children(list_items),
                    )
                }
            }
            FuzzBlock::OrderedList { items } => {
                let list_items: Vec<Node> =
                    items.iter().filter_map(|i| make_list_item(i)).collect();
                if list_items.is_empty() {
                    None
                } else {
                    Some(
                        Node::new(node::LIST)
                            .prop(prop::ORDERED, true)
                            .children(list_items),
                    )
                }
            }
        })
        .collect();

    if content_nodes.is_empty() {
        return;
    }

    let doc = Document::new().with_content(Node::new(node::DOCUMENT).children(content_nodes));

    // Emit to Haddock bytes — must not panic.
    let Ok(emit_result) = rescribe_write_haddock::emit(&doc) else {
        return;
    };
    let _haddock_for_debug = std::str::from_utf8(&emit_result.value)
        .unwrap_or("")
        .to_string();

    // Parse back — must not panic.
    let Ok(haddock_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_haddock::parse(haddock_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "Haddock roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  haddock: {_haddock_for_debug:?}"
    );
});

fn extract_text(node: &Node) -> String {
    let mut text = String::new();
    if node.kind.as_str() == node::TEXT || node.kind.as_str() == node::CODE {
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
    }
    for child in &node.children {
        text.push_str(&extract_text(child));
    }
    text
}
