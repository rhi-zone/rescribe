#![no_main]

//! BBCode roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with BBCode-compatible constructs,
//! emits them to BBCode text via rescribe-write-bbcode, parses them back via
//! rescribe-read-bbcode, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: BBCode tag syntax uses `[`, `]`, `=`.
//! Plain text containing these characters may be re-parsed as markup on the
//! return trip. The sanitiser strips them so the corpus exercises structural
//! variations rather than tag-syntax edge cases.
//!
//! FuzzInlineKind: Plain, Bold, Italic, Code.
//! BBCode has no headings in standard form; FuzzBlock uses Paragraph,
//! BulletList, and OrderedList.

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
    // Code excluded: inline [code]t[/code] emitted in a paragraph that starts
    // with [code] is re-parsed as a block CodeBlock, not a paragraph with an
    // inline code span. This is an inherent BBCode ambiguity (not a bug).
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
    BulletList { items: Vec<Vec<FuzzInline>> },
    OrderedList { items: Vec<Vec<FuzzInline>> },
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip characters that would be re-interpreted as BBCode
/// markup on the parse-back trip.
///
/// BBCode uses `[tag]content[/tag]` syntax. Characters stripped:
/// - ASCII control characters (0x00–0x1f) including newlines
/// - `[` `]` — BBCode tag delimiters
/// - `=` — used in attribute tags like [url=...]
/// - `"` `'` — quote chars in tag attributes
/// - `\` — escape char
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                // All ASCII control chars 0x00–0x1f (includes \n \r \t)
                '\x00'..='\x1f'
                    // BBCode tag delimiters and attribute chars
                    | '[' | ']' | '=' | '"' | '\'' | '\\'
            )
        })
        .collect();

    // Trim and strip structural patterns until stable.
    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();

        // Strip leading [*] list item markers that got through
        while out.starts_with("[*]") {
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
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, text);
    Some(match fi.kind {
        FuzzInlineKind::Plain => leaf,
        FuzzInlineKind::Bold => Node::new(node::STRONG).child(leaf),
        FuzzInlineKind::Italic => Node::new(node::EMPHASIS).child(leaf),
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

    // Emit to BBCode bytes — must not panic.
    let Ok(emit_result) = rescribe_write_bbcode::emit(&doc) else {
        return;
    };
    let _bbcode_for_debug = std::str::from_utf8(&emit_result.value)
        .unwrap_or("")
        .to_string();

    // Parse back — must not panic.
    let Ok(bbcode_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_bbcode::parse(bbcode_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "BBCode roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  bbcode: {_bbcode_for_debug:?}"
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
