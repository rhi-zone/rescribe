#![no_main]

//! Jira wiki roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with Jira-compatible constructs,
//! emits them to Jira wiki text via rescribe-write-jira, parses them back
//! via rescribe-read-jira, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Jira wiki uses:
//! - `*bold*`, `_italic_`, `~subscript~`, `-strikethrough-` for inline markup
//! - `h1. Heading` through `h6. Heading` for headings
//! - `* ` / `# ` for unordered/ordered list items
//! - `[label|url]` for links, `{{code}}` for inline code
//!
//! Characters stripped from text: all ASCII control chars, `*`, `_`, `~`,
//! `-`, `[`, `]`, `{`, `}`, `|`, `#`, `!`, `\\`, `<`, `>`, `+`,
//! `=`, `^`, `%`, `'`, `@`, `` ` ``, `/`.
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

/// Sanitise text: strip characters that would be re-interpreted as Jira wiki markup.
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\x00'..='\x1f'
                    | '*' | '_' | '~' | '-' | '[' | ']' | '{' | '}' | '|' | '#' | '!'
                    | '\\' | '<' | '>' | '+' | '=' | '^' | '%' | '\'' | '@' | '`' | '/'
                    | '.' | ',' | ':' | ';' | '(' | ')'
            )
        })
        .collect();

    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();

        // Strip Jira heading prefixes: "h1. " through "h6. " at start of text
        for level in 1u8..=6 {
            let prefix = format!("h{}. ", level);
            if out.starts_with(&prefix) {
                out = out[prefix.len()..].trim().to_string();
            }
        }

        // Strip ordered list patterns: "N. " or "N) "
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
    let children = make_para_children(inlines);
    if children.is_empty() {
        return None;
    }
    let para = Node::new(node::PARAGRAPH).children(children);
    Some(Node::new(node::LIST_ITEM).child(para))
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

    let Ok(emit_result) = rescribe_write_jira::emit(&doc) else {
        return;
    };
    let _jira_for_debug = std::str::from_utf8(&emit_result.value)
        .unwrap_or("")
        .to_string();

    let Ok(jira_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_jira::parse(jira_str) else {
        return;
    };

    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "Jira roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  jira: {_jira_for_debug:?}"
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
