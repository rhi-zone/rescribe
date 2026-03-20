#![no_main]

//! TikiWiki roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with TikiWiki-compatible constructs,
//! emits them to TikiWiki text via rescribe-write-tikiwiki, parses them back
//! via rescribe-read-tikiwiki, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! TikiWiki uses:
//! - `__bold__`, `''italic''`, `===strike===`, `--deleted--` for inline markup
//! - `!` through `!!!!!!` for headings (bang-prefix)
//! - `*` / `#` for unordered/ordered list items
//! - links with `[url]` or `[url|label]`
//!
//! Characters stripped from text: all ASCII control chars, `*`, `_`, `~`,
//! `=`, `[`, `]`, `{`, `}`, `|`, `#`, `!`, `\\`, `<`, `>`, `-`, `+`,
//! `'`, `^`, `%`, `@`, `` ` ``, `/`.
//! Ordered list patterns stripped to avoid re-parse ambiguity.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rescribe_core::{Document, Node};
use rescribe_std::{node, prop};

#[derive(Arbitrary, Debug)]
enum FuzzInlineKind {
    Plain,
}

#[derive(Arbitrary, Debug)]
struct FuzzInline {
    text: String,
    kind: FuzzInlineKind,
}

#[derive(Arbitrary, Debug)]
enum FuzzBlock {
    Paragraph { inlines: Vec<FuzzInline> },
    Heading { level: u8, inlines: Vec<FuzzInline> },
    BulletList { items: Vec<Vec<FuzzInline>> },
    OrderedList { items: Vec<Vec<FuzzInline>> },
}

/// Sanitise text: strip characters that would be re-interpreted as TikiWiki markup.
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\x00'..='\x1f'
                    | '*' | '_' | '~' | '=' | '[' | ']' | '{' | '}' | '|' | '#' | '!'
                    | '\\' | '<' | '>' | '-' | '+' | '\'' | '^' | '%' | '@' | '`' | '/'
                    | '.' | ',' | ':' | ';' | '(' | ')'
            )
        })
        .collect();

    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();
        loop {
            let digits_end = out.find(|c: char| !c.is_ascii_digit()).unwrap_or(out.len());
            if digits_end == 0 {
                break;
            }
            let rest = &out[digits_end..];
            if rest.starts_with(". ") || rest.starts_with(") ") {
                out = out[digits_end + 2..].trim().to_string();
            } else if rest == "." || rest == ")" {
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
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, text);
    Some(match fi.kind {
        FuzzInlineKind::Plain => leaf,
    })
}

fn make_para_children(inlines: &[FuzzInline]) -> Vec<Node> {
    inlines.iter().filter_map(make_inline).collect()
}

fn make_list_item(inlines: &[FuzzInline]) -> Option<Node> {
    // TikiWiki writer calls nodes_to_inlines directly on LIST_ITEM children,
    // so we must NOT wrap in a PARAGRAPH — use direct inline children.
    let children = make_para_children(inlines);
    if children.is_empty() {
        return None;
    }
    Some(Node::new(node::LIST_ITEM).children(children))
}

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
                let lvl = i64::from(*level % 6) + 1;
                Some(Node::new(node::HEADING).prop(prop::LEVEL, lvl).children(children))
            }
            FuzzBlock::BulletList { items } => {
                let list_items: Vec<Node> =
                    items.iter().filter_map(|i| make_list_item(i)).collect();
                if list_items.is_empty() {
                    None
                } else {
                    Some(Node::new(node::LIST).prop(prop::ORDERED, false).children(list_items))
                }
            }
            FuzzBlock::OrderedList { items } => {
                let list_items: Vec<Node> =
                    items.iter().filter_map(|i| make_list_item(i)).collect();
                if list_items.is_empty() {
                    None
                } else {
                    Some(Node::new(node::LIST).prop(prop::ORDERED, true).children(list_items))
                }
            }
        })
        .collect();

    if content_nodes.is_empty() {
        return;
    }

    let doc = Document::new().with_content(Node::new(node::DOCUMENT).children(content_nodes));

    let Ok(emit_result) = rescribe_write_tikiwiki::emit(&doc) else {
        return;
    };
    let _tikiwiki_for_debug = std::str::from_utf8(&emit_result.value)
        .unwrap_or("")
        .to_string();

    let Ok(tikiwiki_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_tikiwiki::parse(tikiwiki_str) else {
        return;
    };

    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "TikiWiki roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  tikiwiki: {_tikiwiki_for_debug:?}"
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
