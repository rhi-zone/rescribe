#![no_main]

//! RST roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with RST-compatible constructs,
//! emits them to RST text via rescribe-write-rst, parses them back via
//! rescribe-read-rst, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: RST inline markup (`*emphasis*`,
//! ``code``, `_ref_` etc.) is context-sensitive. Plain text containing `*`
//! or backticks may be re-parsed as markup on the return trip. The sanitiser
//! strips these characters so the fuzz corpus exercises structural variations
//! rather than inline-markup edge cases, which are covered by fixtures.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rescribe_core::{Document, Node};
use rescribe_std::{node, prop};

// ── Fuzz-friendly inline types ────────────────────────────────────────────────

#[derive(Arbitrary, Debug)]
enum FuzzInlineKind {
    Plain,
    Bold,
    Italic,
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

/// Sanitise text: strip all ASCII control characters (including tab and
/// newline, which split RST paragraphs) and RST inline markup characters
/// that would be re-interpreted on parse-back.
/// Returns None for empty results.
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                // All ASCII control chars 0x00–0x1f
                '\x00'..='\x1f'
                    // RST inline markup delimiters
                    | '*' | '`' | '_' | '|' | '\\'
                    // '#' is the RST auto-numbered list prefix (#.)
                    | '#'
            )
        })
        .collect();
    // Normalize: trim + strip structural markers until stable.
    // Each rule may expose another (e.g. ".. 5. x" → strip ".. " → "5. x" → strip "5. " → "x";
    // "foo :: " → trim → "foo ::" → strip ":" → "foo" is handled by the outer loop too).
    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        // RST '::' at end of a line triggers literal-block (code block) detection.
        while out.ends_with(':') {
            out.pop();
        }
        out = out.trim().to_string();
        // RST "N. " / ". " enumerated list prefix.
        loop {
            let trimmed = out
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .to_string();
            if trimmed.starts_with(". ") {
                let cut = out.len() - trimmed.len() + 2;
                out = out[cut..].trim().to_string();
            } else if out.starts_with(". ") {
                out = out[2..].trim().to_string();
            } else {
                break;
            }
        }
        // RST "- " / "+ " bullet-list markers ("*" already filtered).
        while out.starts_with("- ") || out.starts_with("+ ") {
            out = out[2..].trim().to_string();
        }
        // RST ".. " directive/comment marker.
        while out.starts_with(".. ") {
            out = out[3..].trim().to_string();
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
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, text.clone());
    Some(match fi.kind {
        FuzzInlineKind::Plain => leaf,
        FuzzInlineKind::Bold => Node::new(node::STRONG).child(leaf),
        FuzzInlineKind::Italic => Node::new(node::EMPHASIS).child(leaf),
        // CODE uses prop::CONTENT directly (not a child TEXT node) — this is
        // how the RST reader produces code nodes, so the writer reads it back.
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
                // Levels 1–5 use safe adornment chars (=, -, ~, ^, ").
                // Level 6+ uses backtick/asterisk/underscore which are RST
                // inline markup and break the roundtrip.
                let lvl = i64::from(*level % 5) + 1; // 1–5
                Some(Node::new(node::HEADING).prop(prop::LEVEL, lvl).children(children))
            }
            FuzzBlock::BulletList { items } => {
                let list_items: Vec<Node> = items.iter().filter_map(|i| make_list_item(i)).collect();
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
                let list_items: Vec<Node> = items.iter().filter_map(|i| make_list_item(i)).collect();
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

    // Emit to RST bytes — must not panic.
    let Ok(emit_result) = rescribe_write_rst::emit(&doc) else {
        return;
    };
    let _rst_for_debug = std::str::from_utf8(&emit_result.value).unwrap_or("").to_string();

    // Parse back — must not panic.
    let Ok(rst_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_rst::parse(rst_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "RST roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  rst: {_rst_for_debug:?}"
    );
});

fn extract_text(node: &Node) -> String {
    let mut text = String::new();
    // TEXT nodes store content in prop::CONTENT; CODE nodes also use prop::CONTENT.
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
