#![no_main]

//! TWiki roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with TWiki-compatible constructs,
//! emits them to TWiki text via rescribe-write-twiki, parses them back
//! via rescribe-read-twiki, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! TWiki uses:
//! - `*bold*`, `_italic_`, `__bolditalic__`, `==boldcode==` for inline markup
//! - `---+ Heading` through `---++++++ Heading` for headings
//! - `   * ` / `   1. ` for unordered/ordered list items (3-space indent)
//! - `[[url][label]]` for links
//!
//! Characters stripped from text: all ASCII control chars, `*`, `_`, `=`,
//! `[`, `]`, `{`, `}`, `|`, `#`, `!`, `\\`, `<`, `>`, `-`, `+`,
//! `~`, `^`, `%`, `'`, `@`, `` ` ``, `/`.
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

/// Sanitise text: strip characters that would be re-interpreted as TWiki markup.
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\x00'..='\x1f'
                    | '*' | '_' | '=' | '[' | ']' | '{' | '}' | '|' | '#' | '!'
                    | '\\' | '<' | '>' | '-' | '+' | '~' | '^' | '%' | '\'' | '@' | '`' | '/'
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
        return None;
    }

    // TWiki WikiWords (CamelCase like WikiWord, GKpeMI) are auto-linked by the parser.
    // A WikiWord is a single word where a lowercase letter is followed by an uppercase letter
    // after position 0. Reject such tokens to prevent roundtrip loss via auto-linking.
    for word in out.split_whitespace() {
        let has_wiki_pattern = {
            let mut saw_lower = false;
            let mut found = false;
            for c in word.chars().skip(1) {
                if c.is_lowercase() { saw_lower = true; }
                if c.is_uppercase() && saw_lower { found = true; break; }
            }
            found
        };
        if has_wiki_pattern {
            return None;
        }
    }

    Some(out)
}

fn make_inline(fi: &FuzzInline) -> Option<Node> {
    let text = sanitise(&fi.text)?;
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, text);
    Some(match fi.kind {
        FuzzInlineKind::Plain => leaf,
    })
}

fn has_wikiword(text: &str) -> bool {
    // A TWiki WikiWord: any word (no spaces) with a lowercase→uppercase transition
    // after position 0. Adjacent text nodes are concatenated by the emitter, so
    // check each whitespace-delimited token in the full text.
    text.split_whitespace().any(|word| {
        let mut saw_lower = false;
        word.chars().skip(1).any(|c| {
            if c.is_lowercase() {
                saw_lower = true;
                false
            } else {
                c.is_uppercase() && saw_lower
            }
        })
    })
}

fn make_para_children(inlines: &[FuzzInline]) -> Vec<Node> {
    let nodes: Vec<Node> = inlines.iter().filter_map(make_inline).collect();
    // TWiki emits adjacent text nodes concatenated. Check the full result for
    // WikiWord patterns that would be auto-linked and thus lost on round-trip.
    let concat: String = nodes
        .iter()
        .filter_map(|n| n.props.get_str(prop::CONTENT).map(str::to_string))
        .collect();
    if has_wikiword(&concat) {
        return vec![];
    }
    nodes
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

    let Ok(emit_result) = rescribe_write_twiki::emit(&doc) else {
        return;
    };
    let _twiki_for_debug = std::str::from_utf8(&emit_result.value)
        .unwrap_or("")
        .to_string();

    let Ok(twiki_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_twiki::parse(twiki_str) else {
        return;
    };

    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "TWiki roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  twiki: {_twiki_for_debug:?}"
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
